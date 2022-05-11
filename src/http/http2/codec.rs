use crate::http::codec::Codec;
use crate::http::error::SomeError;
use crate::http::http2::headers::{flags::END_HEADERS, Headers};
use crate::http::http2::request::Request;
use crate::http::http2::settings::*;
use crate::http::http2::stream::{State, Stream};
use crate::http::http2::window_update::WindowUpdate;
use crate::http::http2::*;
use crate::http::request::RequestBuilder;
use crate::http::utf8::UTF8;
use crate::http::{Error, Response, Result, Success};
use hpack::{Decoder, Encoder};
use rustls::{ClientConnection, StreamOwned};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;

pub struct Http2Codec<'a> {
    pub encoder: Encoder<'a>,
    pub decoder: Decoder<'a>,
    pub last_stream: u32,
    pub settings: StreamSettings,
    pub server_window_size: u32,
    pub client_window_size: u32,
}

impl<'a> Codec for Http2Codec<'a> {
    fn encode_request(&mut self, request: RequestBuilder) -> Result<Vec<u8>> {
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
        self.server_window_size -= encoded.len() as u32 - 9;

        Ok(encoded)
    }
    fn decode_response(
        &mut self,
        conn: &mut StreamOwned<ClientConnection, TcpStream>,
    ) -> Result<Response> {
        let mut stream = Stream::new(self.last_stream);
        while !stream.is_closed() {
            let frame_header = self.expect_frame_header(conn)?;
            if frame_header.length >= self.client_window_size {
                self.update_window(conn)?;
            }
            self.client_window_size -= frame_header.length;
            match frame_header.kind {
                HEADERS => {
                    let headers: HeadersFrame = self.expect_payload(conn, frame_header)?;
                    if headers.is_stream_end() {
                        stream.state = State::Closed;
                    }
                    stream.response_headers.extend(headers.payload.blocks);
                }
                DATA => {
                    let data: DataFrame = self.expect_payload(conn, frame_header)?;
                    if data.is_stream_end() {
                        stream.state = State::Closed;
                    }
                    stream.response_data.extend(data.payload.blocks);
                }
                SETTING => self.update_settings(conn, frame_header)?,
                WINDOW_UPDATE => self.handle_window_update(conn, frame_header)?,
                RST_STREAM => self.handle_stream_reset(conn, frame_header)?,
                GOAWAY => self.handle_go_away(conn, frame_header)?,
                _ => {
                    return Err(Error::connection(
                        "unexpected frame",
                        frame_header.kind.some_box(),
                    ))
                }
            }
        }

        self.decode_stream(stream)
    }

    fn empty_buffer(&self) -> Vec<u8> {
        vec![0; 8192]
    }

    fn handshake(&mut self, conn: &mut StreamOwned<ClientConnection, TcpStream>) -> Success {
        let mut handshake = PREFACE.to_vec();
        handshake.extend(SettingsFrame::empty());
        conn.write_all(&handshake)?;
        let frame: SettingsFrame = self.expect_frame(conn)?;
        self.settings.update(frame.payload);
        Self::ack_settings(conn)?;
        self.update_window(conn)?;

        Ok(())
    }
}

impl<'a> Http2Codec<'a> {
    pub(crate) fn new() -> Self {
        Self {
            encoder: Encoder::new(),
            decoder: Decoder::new(),
            last_stream: 1,
            settings: StreamSettings::default(),
            server_window_size: 65535,
            client_window_size: 65535,
        }
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

    pub fn expect_frame_header(
        &mut self,
        stream: &mut StreamOwned<ClientConnection, TcpStream>,
    ) -> Result<FrameHeader> {
        let mut buffer = [0; 9];
        stream
            .read_exact(&mut buffer)
            .map_err(|e| Error::connection(&e.to_string(), e.some_box()))?;

        Ok(FrameHeader::from(&buffer))
    }

    pub fn expect_payload<P: FramePayload>(
        &mut self,
        stream: &mut StreamOwned<ClientConnection, TcpStream>,
        frame_header: FrameHeader,
    ) -> Result<Frame<P>> {
        let payload = self.try_read_buf(stream, frame_header.length)?;

        Frame::parse_from_payload(frame_header, &payload)
    }

    pub fn expect_frame<P: FramePayload>(
        &mut self,
        stream: &mut StreamOwned<ClientConnection, TcpStream>,
    ) -> Result<Frame<P>> {
        let frame_header = self.expect_frame_header(stream)?;

        self.expect_payload(stream, frame_header)
    }

    pub fn decode_stream(&mut self, stream: Stream) -> Result<Response> {
        let headers = self.decompress_headers(&stream.response_headers)?;
        let status_code = headers
            .get(":status")
            .ok_or_else(|| Error::server("malformed response"))?;
        Ok(Response {
            protocol: Default::default(),
            status_code: self.decode_status(status_code)?,
            headers,
            body: stream.response_data,
        })
    }

    pub fn ack_settings(stream: &mut StreamOwned<ClientConnection, TcpStream>) -> Success {
        stream.write_all(&SettingsFrame::ack())?;

        Ok(())
    }

    pub fn update_settings(
        &mut self,
        stream: &mut StreamOwned<ClientConnection, TcpStream>,
        frame_header: FrameHeader,
    ) -> Success {
        if frame_header.length == 0 && frame_header.flags & 0x1 != 0 {
            return Ok(());
        }
        let frame = self.expect_payload(stream, frame_header)?;
        self.settings.update(frame.payload);

        Self::ack_settings(stream)
    }

    fn handle_window_update(
        &mut self,
        stream: &mut StreamOwned<ClientConnection, TcpStream>,
        frame_header: FrameHeader,
    ) -> Success {
        let frame: WindowUpdateFrame = self.expect_payload(stream, frame_header)?;
        self.server_window_size = frame.payload.window_size_increment;

        Ok(())
    }

    fn update_window(&mut self, conn: &mut StreamOwned<ClientConnection, TcpStream>) -> Success {
        self.client_window_size += self.settings.initial_window_size * 4;
        let frame = WindowUpdate::new(self.client_window_size)
            .to_frame()
            .encode();
        conn.write_all(&frame)?;

        Ok(())
    }

    fn handle_stream_reset(
        &mut self,
        stream: &mut StreamOwned<ClientConnection, TcpStream>,
        frame_header: FrameHeader,
    ) -> Success {
        let frame: RstStreamFrame = self.expect_payload(stream, frame_header)?;

        Err(Error::connection(
            "stream reset by server",
            frame.payload.error_code.some_box(),
        ))
    }

    fn handle_go_away(
        &mut self,
        stream: &mut StreamOwned<ClientConnection, TcpStream>,
        frame_header: FrameHeader,
    ) -> Success {
        let frame: GoAwayFrame = self.expect_payload(stream, frame_header)?;
        let error_message = match frame.payload.additional_debug_data.is_empty() {
            true => "connection reset by server".to_string(),
            false => frame.payload.additional_debug_data.utf8_lossy().to_string(),
        };

        Err(Error::connection(
            &error_message,
            frame.payload.error_code.some_box(),
        ))
    }

    fn try_read_buf<T>(
        &mut self,
        stream: &mut StreamOwned<ClientConnection, TcpStream>,
        size: T,
    ) -> Result<Vec<u8>>
    where
        T: TryInto<usize>,
    {
        let mut buffer = vec![
            0;
            size.try_into().map_err(|_e| Error::client(
                "could not convert buffer length to usize"
            ))?
        ];
        stream
            .read_exact(&mut buffer)
            .map_err(|e| Error::connection(&e.to_string(), e.some_box()))?;

        Ok(buffer)
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
