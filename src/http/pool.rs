use super::{pooled_conn::PooledConn, Error, Response, Result, Success};
use crate::http::error::{ErrorKind, SomeError};
use crate::http::request::RequestBuilder;
use crate::http::utf8_utils::UTF8Utils;
use crate::http::Protocol;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;

pub struct HostPool {
    inner: Option<JoinHandle<()>>,
    request_tx: Sender<RequestBuilder>,
    response_rx: Receiver<Result<Response>>,
}

impl HostPool {
    pub fn new() -> Self {
        let (pool, request_tx, response_rx) = Self::spawn_pool();
        Self {
            inner: Some(pool),
            request_tx,
            response_rx,
        }
    }

    pub fn spawn_pool() -> (
        JoinHandle<()>,
        Sender<RequestBuilder>,
        Receiver<Result<Response>>,
    ) {
        let (request_tx, request_rx) = channel();
        let (response_tx, response_rx) = channel();
        let thread = thread::spawn(move || {
            let mut pool = Pool::new(response_tx);
            let request_rx = request_rx;
            loop {
                let request: RequestBuilder = request_rx.recv().unwrap();
                let host = request.url.host.as_utf8().unwrap();
                let connection = pool.host(&host);
                let connection = match connection {
                    Ok(i) => i,
                    Err(_e) => pool.host(&host).unwrap(),
                };
                if let Err(e) = connection.send_request(request) {
                    pool.clear_connections();
                    panic!("{}", e.to_string());
                }
                let response = connection.check_response();
                if let Err(e) = pool.response_tx.send(response) {
                    pool.clear_connections();
                    panic!("{}", e.to_string());
                } else {
                    pool.check_connections();
                }
            }
        });

        (thread, request_tx, response_rx)
    }

    pub fn send_request(&mut self, request: RequestBuilder) -> Success {
        if let Err(_) = self.request_tx.send(request) {
            let executor = self.inner.take();
            if let Some(executor) = executor {
                if let Err(e) = executor.join() {
                    return Err(Error::new(
                        "connection thread panicked",
                        ErrorKind::Thread(Some(e)),
                    ));
                }
            }
            Err(Error::new(
                "connection pool panicked",
                ErrorKind::Thread(None),
            ))
        } else {
            Ok(())
        }
    }

    pub fn fetch_response(&mut self) -> Result<Response> {
        match self
            .response_rx
            .recv_timeout(std::time::Duration::from_secs(2))
        {
            Ok(i) => i,
            Err(e) => Err(Error::new(
                "could not fetch response",
                ErrorKind::Thread(e.to_string().some_box()),
            )),
        }
    }
}

pub struct Pool {
    map: HashMap<String, PooledConn>,
    pub response_tx: Sender<Result<Response>>,
}

impl Pool {
    pub fn new(response_tx: Sender<Result<Response>>) -> Self {
        Self {
            map: HashMap::with_capacity(4),
            response_tx,
        }
    }
    pub fn spawn_connection(addr: &str) -> Result<PooledConn> {
        PooledConn::new(addr, Protocol::default())
    }

    pub fn host(&mut self, addr: &str) -> Result<&mut PooledConn> {
        let connection = self
            .map
            .entry(addr.to_string())
            .or_insert(Self::spawn_connection(&addr)?);
        match connection.is_active() {
            true => Ok(connection),
            false => Err(Error::new(
                "host connection lost",
                ErrorKind::Thread(addr.to_string().some_box()),
            )),
        }
    }

    pub fn check_connections(&mut self) {
        self.map.retain(|_h, c| c.is_active());
    }

    pub fn clear_connections(&mut self) {
        self.map.iter_mut().for_each(|(_, c)| {
            let _ = c.join_thread();
        });
        self.map.clear();
    }
}
