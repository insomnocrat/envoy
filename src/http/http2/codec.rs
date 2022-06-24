use crate::http::codec::Codec;
use crate::http::error::SomeError;
use crate::http::http2::go_away::GoAway;
use crate::http::http2::headers::Headers;
use crate::http::http2::ping::Ping;
use crate::http::http2::request::Request;
use crate::http::http2::settings::*;
use crate::http::http2::stream::{State, Stream};
use crate::http::http2::window_update::WindowUpdate;
use crate::http::http2::*;
use crate::http::request::RequestBuilder;
use crate::http::utf8_utils::UTF8Utils;
use crate::http::Protocol::HTTP2;
use crate::http::{proto_conn::H2, Error, Protocol, Response, Result, Success};
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
        if request.url.host.eq(b"ping") {
            return Ok(self.encode_ping());
        }
        let request = Request::from(request);
        let mut encoded = self.encode_header_frame(&request.raw_headers, request.data.is_some());
        if let Some(data) = request.data {
            let data_frame = DataFrame::parse_from_payload(
                FrameHeader::new(FrameKind::Data, END_STREAM, self.last_stream),
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
            if frame_header.is_malformed() {
                self.send_go_away(conn)?;
            }
            if frame_header.length >= self.client_window_size {
                self.update_window(conn)?;
            }
            self.client_window_size -= frame_header.length;
            match frame_header.kind {
                FrameKind::Headers => {
                    let headers: HeadersFrame = self.expect_payload(conn, frame_header)?;
                    if headers.payload.is_malformed() {
                        self.send_go_away(conn)?;
                    }
                    if headers.is_stream_end() {
                        stream.state = State::Closed;
                    }
                    stream.response_headers.extend(headers.payload.blocks);
                }
                FrameKind::Data => {
                    let data: DataFrame = self.expect_payload(conn, frame_header)?;
                    if data.payload.is_malformed() {
                        self.send_go_away(conn)?;
                    }
                    if data.is_stream_end() {
                        stream.state = State::Closed;
                    }
                    stream.response_data.extend(data.payload.blocks);
                }
                FrameKind::Continuation => {
                    let continuation: ContinuationFrame =
                        self.expect_payload(conn, frame_header)?;
                    stream.response_headers.extend(continuation.payload.blocks);
                }
                FrameKind::Setting => self.update_settings(conn, frame_header)?,
                FrameKind::WindowUpdate => self.handle_window_update(conn, frame_header)?,
                FrameKind::RstStream => self.handle_stream_reset(conn, frame_header)?,
                FrameKind::GoAway => self.handle_go_away(conn, frame_header)?,
                FrameKind::Ping => return self.receive_ping(conn, frame_header),
                FrameKind::Priority => {}
                FrameKind::PushPromise => {}

                FrameKind::Altsvc => {}
                FrameKind::Origin => {}
            }
        }

        self.decode_stream(stream)
    }

    fn empty_buffer(&self) -> Vec<u8> {
        vec![0; 8192]
    }

    fn prelude(&mut self, conn: &mut StreamOwned<ClientConnection, TcpStream>) -> Success {
        let mut handshake = PREFACE.to_vec();
        handshake.extend(SettingsFrame::empty());
        conn.write_all(&handshake)?;
        conn.flush()?;
        match conn.conn.alpn_protocol() {
            Some(protocol) => {
                if *protocol != *H2 {
                    return Err(Error::protocol("http2 protocol rejected"));
                }
            }
            None => return Err(Error::protocol("alpn protocol not set")),
        }
        let frame: SettingsFrame = self.expect_frame(conn)?;
        self.settings.update(frame.payload);
        Self::ack_settings(conn)?;
        self.update_window(conn)?;

        Ok(())
    }

    fn kind(&self) -> Protocol {
        HTTP2
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
            false => headers::Flags::EndHeaders as u8 | END_STREAM,
            true => headers::Flags::EndHeaders as u8,
        };
        let frame_header = FrameHeader {
            length: encoded.len() as u32,
            kind: FrameKind::Headers,
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
            .map(|(k, v)| (k.as_utf8_lossy().to_string(), v.as_utf8_lossy().to_string()))
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
            status_code: self.decode_status(status_code.as_bytes())?,
            headers,
            body: stream.response_data,
        })
    }

    pub fn ack_settings(stream: &mut StreamOwned<ClientConnection, TcpStream>) -> Success {
        stream.write_all(&SettingsFrame::ack())?;
        stream.flush()?;

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
        let frame: SettingsFrame = self.expect_payload(stream, frame_header)?;
        if frame.payload.is_malformed() {
            self.send_go_away(stream)?;
        }
        self.settings.update(frame.payload);

        Self::ack_settings(stream)
    }

    fn handle_window_update(
        &mut self,
        stream: &mut StreamOwned<ClientConnection, TcpStream>,
        frame_header: FrameHeader,
    ) -> Success {
        let frame: WindowUpdateFrame = self.expect_payload(stream, frame_header)?;
        if frame.payload.is_malformed() {
            self.send_go_away(stream)?;
        }
        self.server_window_size = frame.payload.window_size_increment;

        Ok(())
    }

    fn update_window(&mut self, stream: &mut StreamOwned<ClientConnection, TcpStream>) -> Success {
        self.client_window_size += self.settings.initial_window_size * 4;
        let frame = WindowUpdate::new(self.client_window_size)
            .to_frame()
            .encode();
        stream.write_all(&frame)?;
        stream.flush()?;

        Ok(())
    }

    pub fn encode_ping(&mut self) -> Vec<u8> {
        Ping::new(192837465).to_frame().encode()
    }

    pub fn receive_ping(
        &mut self,
        stream: &mut StreamOwned<ClientConnection, TcpStream>,
        frame_header: FrameHeader,
    ) -> Result<Response> {
        if frame_header.flags & ping::Flags::Ack as u8 == 0x0 {
            return Err(Error::http2(
                "received malformed frame",
                ErrorCode::ProtocolError,
            ));
        }
        let _: PingFrame = self.expect_payload(stream, frame_header)?;
        let received = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("ping traveled backwards in time")
            .as_millis();

        Ok(Response {
            protocol: Default::default(),
            status_code: 200,
            headers: Default::default(),
            body: received.to_be_bytes().to_vec(),
        })
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
            false => frame
                .payload
                .additional_debug_data
                .as_utf8_lossy()
                .to_string(),
        };

        Err(Error::connection(
            &error_message,
            frame.payload.error_code.some_box(),
        ))
    }

    fn send_go_away(&mut self, stream: &mut StreamOwned<ClientConnection, TcpStream>) -> Success {
        let frame = GoAway::new(ErrorCode::ConnectError, None)
            .to_frame()
            .encode();
        stream.write_all(&frame)?;
        stream.flush()?;

        Err(Error::server("received malformed frame"))
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

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
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
            Identifier::HeaderTableSize => self.header_table_size = setting.value,
            Identifier::EnablePush => self.enable_push = setting.value != 0,
            Identifier::InitialWindowSize => self.initial_window_size = setting.value,
            Identifier::MaxConcurrentStreams => self.max_concurrent_streams = setting.value,
            Identifier::MaxFrameSize => self.max_frame_size = setting.value,
            Identifier::MaxHeaderListSize => self.max_header_list_size = setting.value,
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
