use super::buffer::*;
use super::utf8::*;
use super::{request::Request, Error, ErrorKind, Response, Result, Success};
use crate::http::error::SomeError;
use rustls::client::InvalidDnsNameError;
use rustls::ClientConnection as TlsClient;
use rustls::StreamOwned as TlsStream;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::iter::Peekable;
use std::net::TcpStream;
use std::str::FromStr;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::vec::IntoIter;

pub struct Connection {
    pub stream: TlsStream<TlsClient, TcpStream>,
}

impl Connection {
    pub fn new(address: &str, hostname: &str) -> Result<Self> {
        let stream = TcpStream::connect(address)?;
        let tls_client = Self::config_tls(hostname)?;
        let stream = TlsStream::new(tls_client, stream);
        Ok(Self { stream })
    }

    fn config_tls(hostname: &str) -> Result<TlsClient> {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));
        let config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let rc_config = Arc::new(config);
        TlsClient::new(
            rc_config,
            hostname.try_into().map_err(|e: InvalidDnsNameError| {
                Error::new(
                    "invalid host address",
                    ErrorKind::Connection(e.to_string().some_box()),
                )
            })?,
        )
        .map_err(|e| {
            Error::new(
                "could not connect to server",
                ErrorKind::Connection(e.some_box()),
            )
        })
    }

    pub fn write_request(&mut self, message: &[u8]) -> Result<Response> {
        self.stream.write_all(message)?;

        self.assess_response()
    }

    pub fn chunk(&mut self, chunk_size: Vec<u8>, body: &mut Vec<u8>) -> Success {
        let hex = chunk_size.utf8_lossy().to_string();
        let encoded_chunk = i32::from_str_radix(&hex, 16)
            .map_err(|_| Error::new("invalid chunk encoding", ErrorKind::Server))?;
        if encoded_chunk != 0 {
            if encoded_chunk <= (body.len() as i32) {
                if body.utf8_lossy().contains(CHUNK_END) {
                    *body = body[0..(body.len() - 7)].to_vec();
                }
                *body = parse_chunks(&body);
                return Ok(());
            }
            body.extend(self.stream_chunks()?)
        }

        Ok(())
    }

    pub fn assess_response(&mut self) -> Result<Response> {
        let mut buffer = vec![0; 8032];
        if 0 == self.stream.read(&mut buffer)? {
            return Err(Error::new("no server response", ErrorKind::Server));
        }
        let mut buffer = buffer.into_iter().peekable();
        let mut response = self.read_response(&mut buffer)?;
        if let Some(content_length) = response.headers.get("Content-Length") {
            let content_length = u32::from_str(content_length)
                .map_err(|_| Error::new("invalid content length", ErrorKind::Server))?;
            response.body = buffer.trim();
            let len = response.body.len() as u32;
            if len < content_length {
                self.stream_body(&mut response.body, (content_length - len) as usize)?;
            }
        } else if let Some(encoding) = response.headers.get("Transfer-Encoding") {
            if encoding.eq("chunked") {
                let chunk_size = buffer.read_line();
                response.body = buffer.trim();
                if let Some(chunk_size) = chunk_size {
                    self.chunk(chunk_size, &mut response.body)?;
                }
            }
        }

        Ok(response)
    }

    fn stream_body(&mut self, body: &mut Vec<u8>, mut remaining: usize) -> Success {
        let mut buffer = empty_buffer();
        'stream: while 0 != self.stream.read(&mut buffer)? {
            let input = buffer.trim_null();
            remaining -= input.len();
            body.extend(input.as_slice());
            if body.len() >= remaining {
                break 'stream;
            }
            buffer = empty_buffer();
        }

        Ok(())
    }

    fn stream_chunks(&mut self) -> Result<Vec<u8>> {
        let mut body = Vec::with_capacity(8032);
        let mut buffer = empty_buffer();
        'stream: while 0 != self.stream.read(&mut buffer)? {
            if buffer.utf8_lossy().contains(CHUNK_END) {
                let buffer = buffer.trim_null();
                body.extend(&buffer[0..(buffer.len() - 7)]);
                break 'stream;
            }
            body.extend(buffer.trim_null().as_slice());
            buffer = empty_buffer();
        }

        Ok(parse_chunks(&body))
    }

    pub fn read_response(&mut self, bytes: &mut Peekable<IntoIter<u8>>) -> Result<Response> {
        let version = bytes.read_to_space().as_slice().try_into()?;
        let potential_status_code = bytes.read_to_space().utf8()?;
        let status_code = u16::from_str(&potential_status_code)
            .map_err(|_e| Error::new("could not parse status code", ErrorKind::Server))?;
        bytes.read_line();
        let mut headers = HashMap::new();
        while let Some(line) = bytes.read_line() {
            let line = line.utf8()?;
            let (key, value) = line
                .split_once(": ")
                .ok_or_else(|| Error::new("could not parse header", ErrorKind::Server))?;
            headers.insert(key.to_string(), value.to_string());
        }
        while bytes.next_if(|b| *b == CR || *b == LF).is_some() {}

        Ok(Response {
            protocol: version,
            status_code,
            body: vec![],
            headers,
        })
    }
}

fn empty_buffer() -> Vec<u8> {
    vec![0; 8032]
}

fn parse_chunks(bytes: &[u8]) -> Vec<u8> {
    let mut parsed = Vec::with_capacity(bytes.len());
    let mut iter = bytes.iter().peekable();
    while let Some(line) = iter.read_line() {
        if !line.is_hex() {
            parsed.extend(line);
        }
    }

    parsed
}

pub struct ManagedConnection {
    pub host: String,
    pub request_tx: Sender<Request>,
    pub response_rx: Receiver<Result<Response>>,
    pub status: Arc<Mutex<ConnectionStatus>>,
    thread: Option<JoinHandle<()>>,
}

impl ManagedConnection {
    pub fn new(host: &str, hostname: &str) -> Result<Self> {
        let timeout = std::time::Duration::from_secs(30);
        let (request_tx, request_rx): (Sender<Request>, Receiver<Request>) = channel();
        let (response_tx, response_rx) = channel();
        let mut connection = Connection::new(host, hostname)?;
        let status = Arc::new(Mutex::new(ConnectionStatus::ACTIVE));
        let inner_status = status.clone();
        connection
            .stream
            .sock
            .set_read_timeout(Some(timeout))
            .unwrap();
        let thread = thread::spawn(move || {
            let request_rx = request_rx;
            let response_tx = response_tx;
            'inner: loop {
                let request = match request_rx.recv_timeout(timeout) {
                    Ok(i) => i,
                    Err(_e) => {
                        let mut s = inner_status.lock().unwrap();
                        *s = ConnectionStatus::DEAD;
                        break 'inner;
                    }
                };
                let response = connection.write_request(&request.message);
                if let Err(_e) = response_tx.send(response) {
                    let mut s = inner_status.lock().unwrap();
                    *s = ConnectionStatus::DEAD;
                    break 'inner;
                }
            }
        });

        Ok(Self {
            host: host.to_string(),
            request_tx,
            response_rx,
            thread: Some(thread),
            status,
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

    pub fn send_request(&mut self, request: Request) -> Success {
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

#[derive(PartialEq)]
pub enum ConnectionStatus {
    ACTIVE,
    DEAD,
}
