use super::proto_conn::ProtoConn;
use super::{Error, ErrorKind, Response, Result, Success};
use crate::http::error::SomeError;
use crate::http::request::RequestBuilder;
use crate::http::Protocol;
#[cfg(feature = "http2")]
use crate::http::Protocol::{HTTP1, HTTP2};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

#[derive(Debug)]
pub struct PooledConn {
    pub host: String,
    pub request_tx: Sender<RequestBuilder>,
    pub response_rx: Receiver<Result<Response>>,
    pub status: Arc<Mutex<ConnectionStatus>>,
    thread: Option<JoinHandle<ProtoConn>>,
}

impl PooledConn {
    pub fn new(authority: &str, protocol: Protocol) -> Result<Self> {
        let timeout = std::time::Duration::from_secs(30);
        let (request_tx, request_rx): (Sender<RequestBuilder>, Receiver<RequestBuilder>) =
            channel();
        let (response_tx, response_rx) = channel();
        let conn = ProtoConn::new(authority, protocol)?;
        let status = Arc::new(Mutex::new(ConnectionStatus::ACTIVE));
        conn.inner
            .sock
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        let thread = Self::spawn_thread(conn, status.clone(), timeout, request_rx, response_tx);

        Ok(Self {
            host: authority.to_string(),
            request_tx,
            response_rx,
            thread: Some(thread),
            status,
        })
    }

    fn spawn_thread(
        conn: ProtoConn,
        status: Arc<Mutex<ConnectionStatus>>,
        timeout: std::time::Duration,
        request_rx: Receiver<RequestBuilder>,
        response_tx: Sender<Result<Response>>,
    ) -> JoinHandle<ProtoConn> {
        thread::spawn(move || {
            let request_rx = request_rx;
            let response_tx = response_tx;
            let mut connection = conn;
            'inner: loop {
                let request = match request_rx.recv_timeout(timeout) {
                    Ok(i) => i,
                    Err(_) => {
                        let mut s = status.lock().unwrap();
                        *s = ConnectionStatus::DEAD;
                        break 'inner;
                    }
                };
                #[cfg(feature = "http2")]
                if request.protocol == HTTP1 && connection.codec.kind() == HTTP2 {
                    if let Err(_) = connection.downgrade_protocol() {
                        let mut s = status.lock().unwrap();
                        *s = ConnectionStatus::DEAD;
                        break 'inner;
                    }
                }
                let response = connection.send_request(request);
                if let Err(_) = response_tx.send(response) {
                    let mut s = status.lock().unwrap();
                    *s = ConnectionStatus::DEAD;
                    break 'inner;
                }
            }

            connection
        })
    }

    pub fn spawn_connection(&mut self, conn: ProtoConn) {
        let timeout = std::time::Duration::from_secs(30);
        let (request_tx, request_rx): (Sender<RequestBuilder>, Receiver<RequestBuilder>) =
            channel();
        let (response_tx, response_rx) = channel();
        let status = Arc::new(Mutex::new(ConnectionStatus::ACTIVE));
        conn.inner
            .sock
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        let thread = Self::spawn_thread(conn, status.clone(), timeout, request_rx, response_tx);
        self.request_tx = request_tx;
        self.response_rx = response_rx;
        self.status = status;
        self.thread = Some(thread);
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

    pub fn join_thread(&mut self) -> Result<Option<ProtoConn>> {
        let thread = self.thread.take();
        match thread {
            Some(thread) => Ok(Some(thread.join().map_err(|e| {
                Error::thread("could not join connection thread", e.some_box())
            })?)),
            None => Ok(None),
        }
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum ConnectionStatus {
    ACTIVE,
    DEAD,
}
