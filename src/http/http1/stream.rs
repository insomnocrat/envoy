use crate::http::buffer::Buffer;
use crate::http::proto_stream::{Inner, ProtoStream};
use crate::http::utf8::{CHUNK_END, CR, FINAL_CHUNK, LF, UTF8};
use crate::http::{Error, Response, Result, Success};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::iter::Peekable;
use std::str::FromStr;
use std::vec::IntoIter;

pub struct Http1Stream {
    inner: Inner,
}

impl ProtoStream for Http1Stream {
    fn handshake(&mut self) -> Success {
        Ok(())
    }

    fn new(stream: Inner) -> Self {
        Self { inner: stream }
    }

    fn inner(&mut self) -> &mut Inner {
        &mut self.inner
    }

    fn empty_buffer() -> Vec<u8> {
        vec![0; Http1Stream::DEFAULT_BUFFER]
    }
}

impl Http1Stream {
    pub const DEFAULT_BUFFER: usize = 8032;

    pub fn write_request(&mut self, message: &[u8]) -> Result<Response> {
        self.inner.write_all(message)?;

        self.assess_response()
    }

    pub fn assess_response(&mut self) -> Result<Response> {
        let mut buffer = Self::empty_buffer();
        if 0 == self.inner().read(&mut buffer)? {
            return Err(Error::server("no server response"));
        }
        if buffer.starts_with(FINAL_CHUNK) {
            buffer = Self::empty_buffer();
            if 0 == self.inner().read(&mut buffer)? {
                return Err(Error::server("no server response"));
            }
        }
        let mut buffer = buffer.into_iter().peekable();
        let mut response = self.read_response(&mut buffer)?;
        if let Some(content_length) = response.headers.get("Content-Length") {
            let content_length = u32::from_str(content_length)
                .map_err(|_| Error::server("invalid content length"))?;
            response.body = buffer.trim_null();
            if (response.body.len() as u32) < content_length {
                self.stream_body(&mut response.body, content_length as usize)?;
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

    pub fn read_response(&mut self, bytes: &mut Peekable<IntoIter<u8>>) -> Result<Response> {
        let version = bytes.read_to_space().as_slice().try_into()?;
        let potential_status_code = bytes.read_to_space().utf8()?;
        let status_code = u16::from_str(&potential_status_code)
            .map_err(|_e| Error::server("could not parse status code"))?;
        bytes.read_line();
        let mut headers = HashMap::new();
        while let Some(line) = bytes.read_line() {
            let line = line.utf8()?;
            let (key, value) = line
                .split_once(": ")
                .ok_or_else(|| Error::server("could not parse header"))?;
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

    fn stream_body(&mut self, body: &mut Vec<u8>, content_length: usize) -> Success {
        let mut buffer = Self::empty_buffer();
        'stream: while 0 != self.inner.read(&mut buffer)? {
            let input = buffer.trim_null();
            body.extend(input.as_slice());
            if body.len() >= content_length {
                break 'stream;
            }
            buffer = Self::empty_buffer();
        }

        Ok(())
    }

    pub fn chunk(&mut self, chunk_size: Vec<u8>, body: &mut Vec<u8>) -> Success {
        let hex = chunk_size.utf8_lossy().to_string();
        let encoded_chunk =
            i32::from_str_radix(&hex, 16).map_err(|_| Error::server("invalid chunk encoding"))?;
        if encoded_chunk != 0 {
            if encoded_chunk <= (body.len() as i32) {
                if body.utf8_lossy().contains(CHUNK_END) {
                    *body = body[0..(body.len() - 7)].to_vec();
                }
                *body = Self::parse_chunks(&body);
                return Ok(());
            }
            body.extend(self.stream_chunks()?)
        }

        Ok(())
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

    fn stream_chunks(&mut self) -> Result<Vec<u8>> {
        let mut body = Vec::with_capacity(Self::DEFAULT_BUFFER);
        let mut buffer = Self::empty_buffer();
        'stream: while 0 != self.inner.read(&mut buffer)? {
            if buffer.utf8_lossy().contains(CHUNK_END) {
                let buffer = buffer.trim_null();
                body.extend(&buffer[0..(buffer.len() - 7)]);
                break 'stream;
            }
            body.extend(buffer.trim_null().as_slice());
            buffer = Self::empty_buffer();
        }

        Ok(Self::parse_chunks(&body))
    }
}
