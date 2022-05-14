use super::{Method, Protocol};

use crate::http::utf8_utils::UTF8Utils;
use std::collections::HashMap;

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
    pub fn extend_query(&mut self, query: Vec<(&[u8], &[u8])>) {
        let query = query.iter().map(|(k, v)| (k.to_vec(), v.to_vec()));
        self.query.extend(query);
    }

    pub fn query(mut self, query: Vec<(&[u8], &[u8])>) -> Self {
        self.extend_query(query);

        self
    }

    pub fn extend_headers(&mut self, headers: Vec<(&[u8], &[u8])>) {
        let headers = headers.iter().map(|(k, v)| (k.to_vec(), v.to_vec()));
        self.headers.extend(headers);
    }

    pub fn headers(mut self, headers: Vec<(&[u8], &[u8])>) -> Self {
        self.extend_headers(headers);

        self
    }

    pub fn extend_header(&mut self, header: (&[u8], &[u8])) {
        let (key, value) = header;
        self.headers.insert(key.to_lower(), value.to_vec());
    }

    pub fn header(mut self, header: (&[u8], &[u8])) -> Self {
        self.extend_header(header);

        self
    }

    pub fn body_mut(&mut self, body: &[u8]) {
        self.body = Some(body.to_vec());
    }

    pub fn body(self, body: &[u8]) -> Self {
        Self {
            protocol: self.protocol,
            method: self.method,
            url: self.url,
            body: Some(body.to_vec()),
            query: self.query,
            headers: self.headers,
        }
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
        format!("{}:443", self.host.as_utf8_lossy())
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
    pub const HOST: &[u8] = b"Host";
    pub const AUTHORIZATION: &[u8] = b"Authorization";
    pub const CONTENT_LENGTH: &[u8] = b"Content-Length";
    pub const USER_AGENT: &[u8] = b"User-Agent";
    pub const ACCEPT: &[u8] = b"Accept";
    pub const ACCEPT_CHARSET: &[u8] = b"Accept-Charset";
    pub const ACCEPT_LANGUAGE: &[u8] = b"Accept-Language";
    pub const CONNECTION: &[u8] = b"Connection";
    pub const MAX_FORWARDS: &[u8] = b"Max-Forwards";
    pub const FROM: &[u8] = b"From";
    pub const REFERER: &[u8] = b"Referer";
    pub const CONTENT_TYPE: &[u8] = b"Content-Type";
    pub mod values {
        pub const ALL: &[u8] = b"*/*";
        pub const JSON: &[u8] = b"application/json";
        pub const UTF8: &[u8] = b"charset=utf-8";
        pub const TEXT_HTML: &[u8] = b"text/html";
        pub const TEXT_PLAIN: &[u8] = b"text/plain";
        pub const EN_US: &[u8] = b"en_US";
    }
}
