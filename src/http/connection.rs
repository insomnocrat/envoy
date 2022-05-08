use super::proto_stream::ProtoStream;
use super::{request::Http1Request, Error, ErrorKind, Response, Result, Success};
use crate::http::http1::stream::Http1Stream;
// #[cfg(feature = "http2")]
// use crate::http::http2::stream::Http2Stream;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

#[derive(Debug)]
pub struct Connection {
    pub host: String,
    pub request_tx: Sender<Http1Request>,
    pub response_rx: Receiver<Result<Response>>,
    pub status: Arc<Mutex<ConnectionStatus>>,
    thread: Option<JoinHandle<()>>,
}

impl Connection {
    pub fn new(authority: &str) -> Result<Self> {
        let timeout = std::time::Duration::from_secs(30);
        let (request_tx, request_rx): (Sender<Http1Request>, Receiver<Http1Request>) = channel();
        let (response_tx, response_rx) = channel();
        let mut stream = Http1Stream::connect(authority)?;
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
        })
    }

    fn spawn_thread(
        stream: Http1Stream,
        status: Arc<Mutex<ConnectionStatus>>,
        timeout: std::time::Duration,
        request_rx: Receiver<Http1Request>,
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
                let response = connection.write_request(&request.message);
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

    pub fn send_request(&mut self, request: Http1Request) -> Success {
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
