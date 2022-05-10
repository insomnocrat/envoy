use crate::http::http2::headers::{Headers, END_HEADERS};
use crate::http::http2::request::Request;
use crate::http::http2::settings::*;
use crate::http::http2::*;
use crate::http::request::RequestBuilder;
use crate::http::utf8::UTF8;
use crate::http::{Error, Result};
use hpack::{Decoder, Encoder};
use std::collections::HashMap;

pub struct Codec<'a> {
    pub encoder: Encoder<'a>,
    pub decoder: Decoder<'a>,
    pub last_stream: u32,
    pub settings: StreamSettings,
    pub current_window_size: u32,
}

impl<'a> Codec<'a> {
    pub(crate) fn new() -> Self {
        Self {
            encoder: Encoder::new(),
            decoder: Decoder::new(),
            last_stream: 1,
            settings: StreamSettings::default(),
            current_window_size: 65535,
        }
    }

    pub fn encode_request(&mut self, request: RequestBuilder) -> Result<Vec<u8>> {
        let request = Request::from(request);
        let mut encoded = self.encode_header_frame(&request.raw_headers, request.data.is_some());
        if let Some(data) = request.data {
            let data_frame = DataFrame::parse_from_payload(
                FrameHeader::new(DATA, END_STREAM, self.last_stream),
                &data,
            )?;
            encoded.extend(data_frame.encode());
        }
        self.last_stream += 2;
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
            stream_identifier: self.last_stream,
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

#[derive(Debug)]
pub struct StreamSettings {
    pub header_table_size: u32,
    pub enable_push: bool,
    pub max_concurrent_streams: u32,
    pub initial_window_size: u32,
    pub max_frame_size: u32,
    pub max_header_list_size: u32,
}

impl StreamSettings {
    fn update_setting(&mut self, setting: Setting) {
        match setting.identifier {
            SETTINGS_HEADER_TABLE_SIZE => self.header_table_size = setting.value,
            SETTINGS_ENABLE_PUSH => self.enable_push = setting.value != 0,
            SETTINGS_INITIAL_WINDOW_SIZE => self.initial_window_size = setting.value,
            SETTINGS_MAX_CONCURRENT_STREAMS => self.max_concurrent_streams = setting.value,
            SETTINGS_MAX_FRAME_SIZE => self.max_frame_size = setting.value,
            SETTINGS_MAX_HEADER_LIST_SIZE => self.max_header_list_size = setting.value,
            _ => {}
        }
    }

    pub(crate) fn update(&mut self, settings: Vec<Setting>) {
        settings.into_iter().for_each(|s| self.update_setting(s))
    }
}

impl Default for StreamSettings {
    fn default() -> Self {
        Self {
            header_table_size: 4096,
            enable_push: true,
            max_concurrent_streams: 100,
            initial_window_size: 65535,
            max_frame_size: 16384,
            max_header_list_size: 4000,
        }
    }
}
