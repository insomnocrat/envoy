use crate::http::http2::headers::{Headers, END_HEADERS};
use crate::http::http2::request::Request;
use crate::http::http2::*;
use crate::http::request::RequestBuilder;
use crate::http::utf8::UTF8;
use crate::http::{Error, Result};
use hpack::{Decoder, Encoder};
use std::collections::HashMap;

pub struct Codec<'a> {
    pub encoder: Encoder<'a>,
    pub decoder: Decoder<'a>,
    pub stream_id: u32,
    pub current_window_size: u32,
}

impl<'a> Codec<'a> {
    pub(crate) fn new() -> Self {
        Self {
            encoder: Encoder::new(),
            decoder: Decoder::new(),
            stream_id: 1,
            current_window_size: 65535,
        }
    }
    pub fn encode_request(&mut self, request: RequestBuilder) -> Result<Vec<u8>> {
        let request = Request::from(request);
        let mut encoded = self.encode_header_frame(&request.raw_headers, request.data.is_some());
        if let Some(data) = request.data {
            let data_frame = DataFrame::parse_from_payload(
                FrameHeader::new(DATA, END_STREAM, self.stream_id),
                &data,
            )?;
            encoded.extend(data_frame.encode());
        }
        self.stream_id += 2;
        self.current_window_size -= encoded.len() as u32;

        Ok(encoded)
    }
    pub fn encode_header_frame(
        &mut self,
        headers: &[(Vec<u8>, Vec<u8>)],
        has_data: bool,
    ) -> Vec<u8> {
        let encoded = self.compress_headers(headers);
        let flags = match has_data {
            false => END_HEADERS | END_STREAM,
            true => END_HEADERS,
        };
        let frame_header = FrameHeader {
            length: encoded.len() as u32,
            kind: HEADERS,
            flags,
            stream_identifier: self.stream_id,
        };
        let headers = Headers {
            pad_length: None,
            is_exclusive: None,
            stream_dependency: None,
            weight: None,
            blocks: encoded,
            padding: None,
        };

        HeadersFrame::new(frame_header, headers).encode()
    }

    pub fn compress_headers(&mut self, headers: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
        self.encoder.encode(
            headers
                .iter()
                .map(|(k, v)| (k.as_slice(), v.as_slice()))
                .collect::<Vec<(&[u8], &[u8])>>(),
        )
    }

    pub fn decompress_headers(&mut self, headers: &[u8]) -> Result<HashMap<String, String>> {
        Ok(self
            .decoder
            .decode(headers)
            .map_err(|_| Error::server("could not decompress headers"))?
            .into_iter()
            .map(|(k, v)| (k.utf8_lossy().to_string(), v.utf8_lossy().to_string()))
            .collect::<HashMap<String, String>>())
    }
}
