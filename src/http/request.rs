use super::{Method, Protocol};
use crate::http::utf8::{CRLF, SP, UTF8};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Request {
    pub host: String,
    pub message: Vec<u8>,
}

impl Request {
    pub fn host(&self) -> String {
        format!("{}:443", &self.host)
    }
    pub fn hostname(&self) -> &str {
        &self.host
    }
}

#[derive(Clone, Debug)]
pub struct RequestBuilder {
    pub protocol: Protocol,
    pub method: Method,
    pub url: Url,
    pub body: Option<Vec<u8>>,
    pub query: HashMap<Vec<u8>, Vec<u8>>,
    pub headers: HashMap<Vec<u8>, Vec<u8>>,
}

impl RequestBuilder {
    pub fn build(self) -> Request {
        let mut message = Vec::with_capacity(8032);
        match self.method {
            Method::GET => message.extend(b"GET "),
            Method::POST => message.extend(b"POST "),
            Method::PUT => message.extend(b"PUT "),
            Method::PATCH => message.extend(b"PATCH "),
            Method::DELETE => message.extend(b"DELETE "),
        }
        if self.url.resource.is_empty() {
            message.push(SLASH);
        } else {
            message.extend(self.url.resource);
        }
        if !self.query.is_empty() {
            message.push(0x3F);
            for (key, value) in self.query.into_iter() {
                message.extend(key);
                message.push(0x3D);
                message.extend(value);
                message.extend_from_slice(CRLF);
            }
        };
        match self.protocol {
            #[cfg(feature = "http2")]
            Protocol::HTTP2 => message.extend(b" HTTP/2.0\r\n"),
            Protocol::HTTP1 => message.extend(b" HTTP/1.1\r\n"),
        }
        message.extend_from_slice(b"Host: ");
        message.extend_from_slice(&self.url.host);
        message.extend_from_slice(CRLF);
        let body = self.body.unwrap_or_default();
        if !body.is_empty() {
            message.extend_from_slice(b"Content-Length: ");
            message.extend_from_slice(format!("{}\r\n", body.len()).as_bytes());
        }
        let colon = [0x3A, SP];
        for (key, value) in self.headers.into_iter() {
            message.extend(key);
            message.extend_from_slice(&colon);
            message.extend(value);
            message.extend_from_slice(CRLF);
        }
        message.extend(CRLF);
        message.extend(body);

        Request {
            host: self.url.host.utf8_lossy().to_string(),
            message,
        }
    }
    pub fn get(url: &str) -> Self {
        Self {
            protocol: Protocol::default(),
            method: Method::GET,
            url: url.as_bytes().into(),
            body: None,
            query: Default::default(),
            headers: Default::default(),
        }
    }
    pub fn post(url: &str) -> Self {
        Self {
            protocol: Protocol::default(),
            method: Method::POST,
            url: url.as_bytes().into(),
            body: None,
            query: Default::default(),
            headers: Default::default(),
        }
    }
    pub fn put(url: &str) -> Self {
        Self {
            protocol: Protocol::default(),
            method: Method::PUT,
            url: url.as_bytes().into(),
            body: None,
            query: Default::default(),
            headers: Default::default(),
        }
    }
    pub fn patch(url: &str) -> Self {
        Self {
            protocol: Protocol::default(),
            method: Method::PATCH,
            url: url.as_bytes().into(),
            body: None,
            query: Default::default(),
            headers: Default::default(),
        }
    }
    pub fn delete(url: &str) -> Self {
        Self {
            protocol: Protocol::default(),
            method: Method::DELETE,
            url: url.as_bytes().into(),
            body: None,
            query: Default::default(),
            headers: Default::default(),
        }
    }
    pub fn query(&mut self, query: Vec<(&[u8], &[u8])>) {
        let query = query.iter().map(|(k, v)| (k.to_vec(), v.to_vec()));
        self.query.extend(query);
    }

    pub fn headers(&mut self, headers: Vec<(&[u8], &[u8])>) {
        let headers = headers.iter().map(|(k, v)| (k.to_vec(), v.to_vec()));
        self.headers.extend(headers);
    }

    pub fn header(&mut self, header: (&[u8], &[u8])) {
        let (key, value) = header;
        self.headers.insert(key.to_vec(), value.to_vec());
    }

    pub fn body(&mut self, body: &[u8]) {
        self.body = Some(body.to_vec());
    }

    pub(crate) fn template(&self) -> Request {
        self.clone().build()
    }
}

const SCHEME: &[u8] = b"https://";
const AUTHORITY: &[u8] = b"www.";
const SLASH: u8 = 0x2f;

#[derive(Clone, Debug)]
pub struct Url {
    pub host: Vec<u8>,
    pub resource: Vec<u8>,
}

impl From<&[u8]> for Url {
    fn from(mut value: &[u8]) -> Self {
        if value.starts_with(SCHEME) {
            value = &value[7..];
        }
        if value.starts_with(AUTHORITY) {
            value = &value[3..];
        }
        let mut host = vec![];
        let mut resource = vec![];
        let mut value = value.iter().peekable();
        while let Some(byte) = value.next_if(|b| **b != SLASH) {
            host.push(*byte);
        }
        resource.extend(value);

        Self { host, resource }
    }
}
