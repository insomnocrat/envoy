use crate::http::error::SomeError;
use crate::http::http2::codec::Codec;
use crate::http::http2::settings::*;
use crate::http::http2::*;
use crate::http::proto_stream::{Inner, ProtoStream};
use crate::http::request::RequestBuilder;
use crate::http::utf8::UTF8;
use crate::http::{Error, Response, Result, Success};

use std::io::{Read, Write};
use std::str::FromStr;

pub struct Http2Stream<'a> {
    pub(crate) inner: Inner,
    pub(crate) codec: Codec<'a>,
    pub(crate) settings: StreamSettings,
}

impl<'a> ProtoStream for Http2Stream<'a> {
    const ALPN_PROTOCOLS: Option<&'static [&'static [u8]]> = Some(&[b"h2", b"http/1.1"]);

    fn handshake(&mut self) -> Success {
        let mut handshake = PREFACE.to_vec();
        handshake.extend(SettingsFrame::empty());
        self.inner.write_all(&handshake)?;
        let frame: SettingsFrame = self.expect_settings()?;
        self.settings.update_bulk(frame.payload);
        self.ack_settings()?;

        Ok(())
    }

    fn new(stream: Inner) -> Self {
        Self {
            inner: stream,
            codec: Codec::new(),
            settings: StreamSettings::default(),
        }
    }

    fn inner(&mut self) -> &mut Inner {
        &mut self.inner
    }

    fn empty_buffer() -> Vec<u8> {
        vec![0; Http2Stream::DEFAULT_BUFFER]
    }

    fn send_request(&mut self, request: RequestBuilder) -> Result<Response> {
        let request = self.codec.encode_request(request)?;
        self.inner.write_all(&request)?;

        self.expect_response()
    }
}

impl<'a> Http2Stream<'a> {
    pub const DEFAULT_BUFFER: usize = 8192;

    pub fn parse_frame_header(&mut self) -> Result<FrameHeader> {
        let mut buffer = [0; 9];
        self.inner
            .read_exact(&mut buffer)
            .map_err(|_| Error::server("expected frame header"))?;

        Ok(FrameHeader::from(&buffer))
    }

    pub fn parse_payload<P, S>(&mut self, size: S, flags: u8) -> Result<P>
    where
        P: FramePayload,
        S: Into<usize>,
    {
        P::parse(self.read_buf(size)?.as_slice(), flags)
    }

    pub fn try_parse_payload<P, S>(&mut self, size: S, flags: u8) -> Result<P>
    where
        P: FramePayload,
        S: TryInto<usize>,
    {
        P::parse(self.try_read_buf(size)?.as_slice(), flags)
    }

    pub fn expect_settings(&mut self) -> Result<SettingsFrame> {
        let header = self.parse_frame_header()?;
        let payload = self.try_parse_payload(header.length, header.flags)?;

        Ok(Frame::new(header, payload))
    }

    pub fn parse_response_headers(&mut self, frame_header: FrameHeader) -> Result<HeadersFrame> {
        let payload = self.try_read_buf(frame_header.length)?;

        HeadersFrame::parse_from_payload(frame_header, &payload)
    }

    pub fn expect_data(&mut self) -> Result<DataFrame> {
        let header = self.parse_frame_header()?;

        self.parse_frame(header)
    }

    pub fn parse_response(&mut self, frame_header: FrameHeader) -> Result<Response> {
        let headers: HeadersFrame = self.parse_frame(frame_header)?;
        let mut body = Vec::new();
        if headers.header.flags & END_STREAM == 0 {
            loop {
                let data = self.expect_data()?;
                let ended = data.header.flags & END_STREAM != 0;
                body.extend(data.payload.blocks);
                if ended {
                    break;
                }
            }
        }
        let headers = self.codec.decompress_headers(&headers.payload.blocks)?;
        let status_code = headers
            .get(":status")
            .ok_or_else(|| Error::server("malformed response"))?;

        Ok(Response {
            protocol: Default::default(),
            status_code: u16::from_str(&status_code)
                .map_err(|_e| Error::server("could not parse status code"))?,
            headers,
            body,
        })
    }

    fn expect_response(&mut self) -> Result<Response> {
        loop {
            let frame_header = self.parse_frame_header()?;
            match frame_header.kind == HEADERS {
                true => return self.parse_response(frame_header),
                false => {
                    self.assess_misc_frame(frame_header)?;
                    continue;
                }
            }
        }
    }

    pub fn ack_settings(&mut self) -> Success {
        self.inner.write_all(&SettingsFrame::ack())?;

        Ok(())
    }

    pub fn parse_frame<P: FramePayload>(&mut self, frame_header: FrameHeader) -> Result<Frame<P>> {
        let payload = self.read_payload(&frame_header)?;

        Frame::parse_from_payload(frame_header, &payload)
    }

    pub fn parse_settings(&mut self, frame_header: FrameHeader) -> Result<SettingsFrame> {
        let payload = self.try_read_buf(frame_header.length)?;

        SettingsFrame::parse_from_payload(frame_header, &payload)
    }

    pub fn parse_wu(&mut self, frame_header: FrameHeader) -> Result<WindowUpdateFrame> {
        let payload = self.try_read_buf(frame_header.length).unwrap();

        WindowUpdateFrame::parse_from_payload(frame_header, &payload)
    }

    fn read_payload(&mut self, frame_header: &FrameHeader) -> Result<Vec<u8>> {
        self.try_read_buf(frame_header.length)
    }

    fn update_settings(&mut self, frame_header: FrameHeader) -> Success {
        if frame_header.length == 0 && frame_header.flags & 0x1 != 0 {
            return Ok(());
        }
        let frame = self.parse_settings(frame_header)?;
        self.settings.update_bulk(frame.payload);

        self.ack_settings()
    }

    fn update_window(&mut self, frame_header: FrameHeader) -> Success {
        let frame = self.parse_wu(frame_header)?;
        self.codec.current_window_size = frame.payload.window_size_increment;

        Ok(())
    }

    fn handle_stream_reset(&mut self, frame_header: FrameHeader) -> Success {
        let frame: RstStreamFrame = self.parse_frame(frame_header)?;

        Err(Error::connection(
            "stream reset by server",
            frame.payload.error_code.some_box(),
        ))
    }

    fn handle_go_away(&mut self, frame_header: FrameHeader) -> Success {
        let frame: GoAwayFrame = self.parse_frame(frame_header)?;
        let error_message = match frame.payload.additional_debug_data.is_empty() {
            true => "connection reset by server".to_string(),
            false => frame.payload.additional_debug_data.utf8_lossy().to_string(),
        };

        Err(Error::connection(
            &error_message,
            frame.payload.error_code.some_box(),
        ))
    }

    pub fn assess_misc_frame(&mut self, frame_header: FrameHeader) -> Success {
        match frame_header.kind {
            SETTING => self.update_settings(frame_header),
            WINDOW_UPDATE => self.update_window(frame_header),
            RST_STREAM => self.handle_stream_reset(frame_header),
            GOAWAY => self.handle_go_away(frame_header),
            _ => Err(Error::connection(
                "unexpected frame",
                frame_header.kind.some_box(),
            )),
        }
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
    fn update(&mut self, setting: Setting) {
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

    fn update_bulk(&mut self, settings: Vec<Setting>) {
        settings.into_iter().for_each(|s| self.update(s))
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
