use super::proto_stream::ProtoStream;
use super::{Error, ErrorKind, Response, Result, Success};
use std::marker::PhantomData;
// #[cfg(feature = "http2")]
// use crate::http::http2::stream::Http2Stream;
use crate::http::request::RequestBuilder;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

#[derive(Debug)]
pub struct Connection<S: ProtoStream> {
    pub host: String,
    pub request_tx: Sender<RequestBuilder>,
    pub response_rx: Receiver<Result<Response>>,
    pub status: Arc<Mutex<ConnectionStatus>>,
    thread: Option<JoinHandle<()>>,
    _stream: PhantomData<S>,
}

impl<S: 'static + ProtoStream> Connection<S> {
    pub fn new(authority: &str) -> Result<Self> {
        let timeout = std::time::Duration::from_secs(30);
        let (request_tx, request_rx): (Sender<RequestBuilder>, Receiver<RequestBuilder>) =
            channel();
        let (response_tx, response_rx) = channel();
        let mut stream = S::connect(authority)?;
        stream.handshake()?;
        let status = Arc::new(Mutex::new(ConnectionStatus::ACTIVE));
        stream.inner().sock.set_read_timeout(Some(timeout)).unwrap();
        let thread = Self::spawn_thread(stream, status.clone(), timeout, request_rx, response_tx);

        Ok(Self {
            host: authority.to_string(),
            request_tx,
            response_rx,
            thread: Some(thread),
            status,
            _stream: PhantomData,
        })
    }

    fn spawn_thread(
        stream: S,
        status: Arc<Mutex<ConnectionStatus>>,
        timeout: std::time::Duration,
        request_rx: Receiver<RequestBuilder>,
        response_tx: Sender<Result<Response>>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let request_rx = request_rx;
            let response_tx = response_tx;
            let mut connection = stream;
            'inner: loop {
                let request = match request_rx.recv_timeout(timeout) {
                    Ok(i) => i,
                    Err(_e) => {
                        let mut s = status.lock().unwrap();
                        *s = ConnectionStatus::DEAD;
                        break 'inner;
                    }
                };
                let response = connection.send_request(request);
                if let Err(_e) = response_tx.send(response) {
                    let mut s = status.lock().unwrap();
                    *s = ConnectionStatus::DEAD;
                    break 'inner;
                }
            }
        })
    }

    pub fn check_response(&mut self) -> Result<Response> {
        match self.response_rx.recv() {
            Ok(i) => i,
            Err(e) => Err(Error::new(
                "could not retrieve request",
                ErrorKind::Thread(Some(Box::new(e.to_string()))),
            )),
        }
    }

    pub fn send_request(&mut self, request: RequestBuilder) -> Success {
        self.request_tx.send(request).map_err(|e| {
            Error::new(
                "could not send request",
                ErrorKind::Thread(Some(Box::new(e.to_string()))),
            )
        })
    }
    pub fn is_active(&mut self) -> bool {
        let status = self.status.lock().unwrap();
        *status == ConnectionStatus::ACTIVE
    }

    pub fn is_dead(&mut self) -> bool {
        let status = self.status.lock().unwrap();
        *status == ConnectionStatus::DEAD
    }

    pub fn join_thread(&mut self) {
        let thread = self.thread.take();
        if let Some(thread) = thread {
            let _ = thread.join();
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum ConnectionStatus {
    ACTIVE,
    DEAD,
}
