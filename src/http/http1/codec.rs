use crate::http::codec::Codec;
use crate::http::request::RequestBuilder;
use crate::http::utf8_utils::{UTF8Parser, UTF8Utils, COLSP, CRLF};
use crate::http::Protocol::HTTP1;
use crate::http::{Error, Method, Protocol, Response, Result, Success};
use crate::rest::request::{CONTENT_LENGTH, HOST};
use rustls::ClientConnection as TlsClient;
use rustls::StreamOwned as TlsStream;
use std::collections::HashMap;
use std::io::Read;
use std::net::TcpStream;
use std::str::FromStr;

pub const DANGLING_CHUNK: &[u8; 3] = b"\r\n0";
pub const CHUNK_END: &str = "\r\n0\r\n\r\n";
pub const FINAL_CHUNK: &[u8] = b"0\r\n\r\n";

#[derive(Debug, Clone, Copy)]
pub struct Http1Codec;

impl Codec for Http1Codec {
    fn encode_request(&mut self, request: RequestBuilder) -> Result<Vec<u8>> {
        let mut message = Vec::with_capacity(8032);
        match request.method {
            Method::GET => message.extend(b"GET "),
            Method::POST => message.extend(b"POST "),
            Method::PUT => message.extend(b"PUT "),
            Method::PATCH => message.extend(b"PATCH "),
            Method::DELETE => message.extend(b"DELETE "),
        }
        if request.url.resource.is_empty() {
            message.push(0x2f);
        } else {
            message.extend(request.url.resource);
        }
        if !request.query.is_empty() {
            message.push(0x3F);
            for (key, value) in request.query.into_iter() {
                message.extend(key);
                message.push(0x3D);
                message.extend(value);
            }
        };
        message.extend(b" HTTP/1.1\r\n");
        message.extend_from_slice(HOST);
        message.extend_from_slice(COLSP);
        message.extend_from_slice(&request.url.host);
        message.extend_from_slice(CRLF);
        for (key, value) in request.headers.into_iter() {
            message.extend(key);
            message.extend_from_slice(COLSP);
            message.extend(value);
            message.extend_from_slice(CRLF);
        }
        let body = request.body.unwrap_or_default();
        if !body.is_empty() {
            message.extend_from_slice(CONTENT_LENGTH);
            message.extend_from_slice(COLSP);
            message.extend_from_slice(format!("{}\r\n", body.len()).as_bytes());
        }
        message.extend(CRLF);
        message.extend(body);

        Ok(message)
    }

    fn decode_response(
        &mut self,
        stream: &mut TlsStream<TlsClient, TcpStream>,
    ) -> Result<Response> {
        let mut buffer = self.empty_buffer();
        if 0 == stream.read(&mut buffer)? {
            return Err(Error::server("no server response"));
        }
        if buffer.starts_with(FINAL_CHUNK) {
            buffer = self.empty_buffer();
            if 0 == stream.read(&mut buffer)? {
                return Err(Error::server("no server response"));
            }
        }
        let mut parser = buffer.into_utf8_parser();
        let mut response = self.decode_response_headers(&mut parser)?;
        if let Some(content_length) = response.headers.get("Content-Length") {
            let content_length = Self::parse_content_length(content_length)?;
            parser.read_to_end(&mut response.body)?;
            if (response.body.len() as u32) < content_length {
                self.stream_body(stream, &mut response.body, content_length as usize)?;
            }
        } else if let Some(encoding) = response.headers.get("Transfer-Encoding") {
            if encoding.eq("chunked") {
                let mut chunk_size = Vec::with_capacity(5);
                parser.read_to_crlf(&mut chunk_size)?;
                response.body = parser.to_vec().trim_crlf().trim_chars(DANGLING_CHUNK);
                if chunk_size.len() != 0 {}
                self.chunk(stream, chunk_size, &mut response.body)?;
            }
        }

        Ok(response)
    }

    fn empty_buffer(&self) -> Vec<u8> {
        vec![0; 8032]
    }

    fn prelude(&mut self, _stream: &mut TlsStream<TlsClient, TcpStream>) -> Success {
        Ok(())
    }

    fn kind(&self) -> Protocol {
        HTTP1
    }
}

impl Http1Codec {
    pub fn new() -> Self {
        Self
    }
    pub fn parse_content_length(cl: &str) -> Result<u32> {
        u32::from_str(cl).map_err(|_| Error::server("invalid content length"))
    }

    pub fn decode_response_headers(&self, parser: &mut UTF8Parser) -> Result<Response> {
        let mut version = Vec::with_capacity(5);
        parser.read_to_space(&mut version)?;
        let version = version.as_slice().try_into()?;
        let mut potential_status_code = Vec::with_capacity(8);
        parser.read_to_space(&mut potential_status_code)?;
        let status_code = self.decode_status(&potential_status_code)?;
        parser.skip_to_crlf();
        let mut headers = HashMap::new();
        let header_lines = parser.take_crlf_strings();
        for line in header_lines.into_iter() {
            let (key, value) = line
                .split_once(": ")
                .ok_or_else(|| Error::server("could not parse header"))?;
            headers.insert(key.to_string(), value.to_string());
        }
        parser.skip_chars(CRLF);

        Ok(Response {
            protocol: version,
            status_code,
            body: vec![],
            headers,
        })
    }

    fn parse_chunks(bytes: &[u8]) -> Vec<u8> {
        let mut parser = bytes.into_utf8_parser();
        let mut chunks = Vec::with_capacity(bytes.len());
        let lines = parser.take_crlf();
        for line in lines {
            if !line.is_hex() {
                chunks.extend(line)
            }
        }

        chunks
    }

    fn stream_body(
        &self,
        stream: &mut TlsStream<TlsClient, TcpStream>,
        body: &mut Vec<u8>,
        content_length: usize,
    ) -> Success {
        let mut buffer = self.empty_buffer();
        'stream: while 0 != stream.read(&mut buffer)? {
            let input = buffer.strip_null();
            body.extend(input.as_slice());
            if body.len() >= content_length {
                break 'stream;
            }
            buffer = self.empty_buffer();
        }

        Ok(())
    }

    pub fn chunk(
        &self,
        stream: &mut TlsStream<TlsClient, TcpStream>,
        chunk_size: Vec<u8>,
        body: &mut Vec<u8>,
    ) -> Success {
        let hex = chunk_size.as_utf8_lossy().to_string();
        let encoded_chunk =
            i32::from_str_radix(&hex, 16).map_err(|_| Error::server("invalid chunk encoding"))?;
        if encoded_chunk != 0 {
            if encoded_chunk <= (body.len() as i32) {
                if body.as_utf8_lossy().contains(CHUNK_END) {
                    *body = body[0..(body.len() - 7)].to_vec();
                }
                *body = Self::parse_chunks(&body);
                return Ok(());
            }
            body.extend(self.stream_chunks(stream)?)
        }

        Ok(())
    }

    fn stream_chunks(&self, stream: &mut TlsStream<TlsClient, TcpStream>) -> Result<Vec<u8>> {
        let mut buffer = self.empty_buffer();
        let mut body = Vec::with_capacity(buffer.len());
        'stream: while 0 != stream.read(&mut buffer)? {
            if buffer.as_utf8_lossy().contains(CHUNK_END) {
                let buffer = buffer.strip_null();
                body.extend(&buffer[0..(buffer.len() - 7)]);
                break 'stream;
            }
            body.extend(buffer.strip_null().as_slice());
            buffer = self.empty_buffer();
        }

        Ok(Self::parse_chunks(&body))
    }
}
