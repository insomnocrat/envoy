use super::{Method, Protocol};

use crate::http::utf8::{CRLF, SP, UTF8};
use headers::*;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Http1Request {
    pub host: String,
    pub message: Vec<u8>,
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
    pub fn build_http1(self) -> Http1Request {
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
        message.extend(b" HTTP/1.1\r\n");
        message.extend_from_slice(HOST);
        message.extend_from_slice(&self.url.host);
        message.extend_from_slice(CRLF);
        let body = self.body.unwrap_or_default();
        if !body.is_empty() {
            message.extend_from_slice(CONTENT_LENGTH);
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

        Http1Request {
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
}

const SCHEME: &[u8] = b"https://";
const AUTHORITY: &[u8] = b"www.";
const SLASH: u8 = 0x2f;

#[derive(Clone, Debug)]
pub struct Url {
    pub host: Vec<u8>,
    pub resource: Vec<u8>,
}

impl Url {
    pub(crate) fn authority(&self) -> String {
        format!("{}:443", self.host.utf8_lossy())
    }
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

pub mod headers {
    pub const HOST: &[u8] = b"Host: ";
    pub const AUTHORIZATION: &[u8] = b"Authorization: ";
    pub const CONTENT_LENGTH: &[u8] = b"Content-Length: ";
    pub const USER_AGENT: &[u8] = b"User-Agent: ";
    pub const ACCEPT: &[u8] = b"Accept: ";
    pub const ACCEPT_CHARSET: &[u8] = b"Accept-Charset: ";
    pub const ACCEPT_LANGUAGE: &[u8] = b"Accept-Language: ";
    pub const CONNECTION: &[u8] = b"Connection: ";
    pub const MAX_FORWARDS: &[u8] = b"Max-Forwards: ";
    pub const FROM: &[u8] = b"From: ";
    pub const REFERER: &[u8] = b"Referer: ";
    pub mod values {
        pub const ALL: &[u8] = b"*/*";
        pub const JSON: &[u8] = b"application/json";
        pub const UTF8: &[u8] = b"charset=utf-8";
        pub const TEXT_HTML: &[u8] = b"text/html";
        pub const TEXT_PLAIN: &[u8] = b"text/plain";
        pub const EN_US: &[u8] = b"en_US";
    }
}
