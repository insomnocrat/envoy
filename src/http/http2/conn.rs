use crate::http::http2::stream::{State, Stream};
use crate::http::{
    error::SomeError,
    http2::{codec::Codec, *},
    proto_conn::{Inner, ProtoConn},
    request::RequestBuilder,
    utf8::UTF8,
    Error, Response, Result, Success,
};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::str::FromStr;

pub struct Http2Conn<'a> {
    pub(crate) inner: Inner,
    pub(crate) codec: Codec<'a>,
    pub(crate) streams: HashMap<u32, Stream>,
}

impl<'a> ProtoConn for Http2Conn<'a> {
    const ALPN_PROTOCOLS: Option<&'static [&'static [u8]]> = Some(&[b"h2", b"http/1.1"]);

    fn handshake(&mut self) -> Success {
        let mut handshake = PREFACE.to_vec();
        handshake.extend(SettingsFrame::empty());
        self.inner.write_all(&handshake)?;
        let frame: SettingsFrame = self.expect_frame()?;
        self.codec.settings.update(frame.payload);
        self.ack_settings()?;

        Ok(())
    }

    fn new(stream: Inner) -> Self {
        let codec = Codec::new();
        let streams = HashMap::with_capacity(codec.settings.max_concurrent_streams as usize);
        Self {
            inner: stream,
            codec,
            streams,
        }
    }

    fn inner(&mut self) -> &mut Inner {
        &mut self.inner
    }

    fn empty_buffer() -> Vec<u8> {
        vec![0; Http2Conn::DEFAULT_BUFFER]
    }

    fn send_request(&mut self, request: RequestBuilder) -> Result<Response> {
        let encoded = self.codec.encode_request(request)?;
        self.inner.write_all(&encoded)?;

        self.expect_response()
    }

    // fn send_request(&mut self, requests: Vec<RequestBuilder>) -> Result<Vec<Result<Response>>> {
    //     let mut results = Vec::new();
    //     let mut closed_streams = Vec::new();
    //     let streams = self.codec.encode_multiplexed(requests)?;
    //     let mut requests: Vec<u8> = Vec::with_capacity(self.codec.settings.max_frame_size as usize);
    //     for stream in streams.into_iter() {
    //         let mut stream = stream?;
    //         let request = stream.request.take().unwrap();
    //         requests.extend(request);
    //         self.streams.insert(stream.id, stream);
    //     }
    //     self.inner.write_all(&requests)?;
    //     let mut waiting = true;
    //     while waiting {
    //         self.asses_frame()?;
    //         let closed = self.close_streams();
    //         closed
    //             .into_iter()
    //             .for_each(|stream| closed_streams.push(stream));
    //         waiting = !self.streams.is_empty()
    //     }
    //     closed_streams
    //         .into_iter()
    //         .for_each(|stream| results.push(self.assemble_response(stream)));
    //
    //     Ok(results)
    // }
}

impl<'a> Http2Conn<'a> {
    pub const DEFAULT_BUFFER: usize = 8192;

    pub fn close_streams(&mut self) -> Vec<Stream> {
        let closing = self
            .streams
            .iter()
            .filter(|(id, stream)| stream.is_closed())
            .map(|(id, stream)| *id)
            .collect::<Vec<u32>>();

        closing
            .into_iter()
            .map(|id| self.streams.remove(&id).unwrap())
            .collect::<Vec<Stream>>()
    }

    pub fn asses_frame(&mut self) -> Success {
        let frame_header = self.expect_frame_header()?;
        match frame_header.kind {
            HEADERS => {
                let headers: HeadersFrame = self.expect_payload(frame_header)?;
                let stream = self.get_stream_mut(headers.id())?;
                if headers.is_stream_end() {
                    stream.state = State::Closed;
                }
                stream.response_headers.extend(headers.payload.blocks);

                Ok(())
            }
            DATA => {
                let data: DataFrame = self.expect_payload(frame_header)?;
                let stream = self.get_stream_mut(data.id())?;
                if data.is_stream_end() {
                    stream.state = State::Closed;
                }
                stream.response_data.extend(data.payload.blocks);

                Ok(())
            }
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

    pub fn get_stream(&mut self, id: &u32) -> Result<&Stream> {
        self.streams
            .get(id)
            .ok_or_else(|| Error::client("placeholder"))
    }

    pub fn get_stream_mut(&mut self, id: &u32) -> Result<&mut Stream> {
        self.streams
            .get_mut(id)
            .ok_or_else(|| Error::client("placeholder"))
    }

    pub fn expect_frame_header(&mut self) -> Result<FrameHeader> {
        let mut buffer = [0; 9];
        self.inner
            .read_exact(&mut buffer)
            .map_err(|_| Error::server("expected frame header"))?;

        Ok(FrameHeader::from(&buffer))
    }

    pub fn expect_payload<P: FramePayload>(
        &mut self,
        frame_header: FrameHeader,
    ) -> Result<Frame<P>> {
        let payload = self.try_read_buf(frame_header.length)?;

        Frame::parse_from_payload(frame_header, &payload)
    }

    pub fn expect_frame<P: FramePayload>(&mut self) -> Result<Frame<P>> {
        let frame_header = self.expect_frame_header()?;
        self.expect_payload(frame_header)
    }

    pub fn parse_response_headers(&mut self, frame_header: FrameHeader) -> Result<HeadersFrame> {
        let payload = self.try_read_buf(frame_header.length)?;

        HeadersFrame::parse_from_payload(frame_header, &payload)
    }

    pub fn parse_response(&mut self, frame_header: FrameHeader) -> Result<Response> {
        let headers: HeadersFrame = self.expect_payload(frame_header)?;
        let mut body = Vec::new();
        if headers.header.flags & END_STREAM == 0 {
            loop {
                let data: DataFrame = self.expect_frame()?;
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

    pub fn assemble_response(&mut self, stream: Stream) -> Result<Response> {
        let headers = self.codec.decompress_headers(&stream.response_headers)?;
        let status_code = headers
            .get(":status")
            .ok_or_else(|| Error::server("malformed response"))?;
        Ok(Response {
            protocol: Default::default(),
            status_code: u16::from_str(&status_code)
                .map_err(|_e| Error::server("could not parse status code"))?,
            headers,
            body: stream.response_data,
        })
    }

    fn expect_response(&mut self) -> Result<Response> {
        loop {
            let frame_header = self.expect_frame_header()?;
            match frame_header.kind == HEADERS {
                true => return self.parse_response(frame_header),
                false => {
                    self.handle_misc_frames(frame_header)?;
                    continue;
                }
            }
        }
    }

    pub fn ack_settings(&mut self) -> Success {
        self.inner.write_all(&SettingsFrame::ack())?;

        Ok(())
    }

    fn update_settings(&mut self, frame_header: FrameHeader) -> Success {
        if frame_header.length == 0 && frame_header.flags & 0x1 != 0 {
            return Ok(());
        }
        let frame = self.expect_payload(frame_header)?;
        self.codec.settings.update(frame.payload);

        self.ack_settings()
    }

    fn update_window(&mut self, frame_header: FrameHeader) -> Success {
        let frame: WindowUpdateFrame = self.expect_payload(frame_header)?;
        self.codec.current_window_size = frame.payload.window_size_increment;

        Ok(())
    }

    fn handle_stream_reset(&mut self, frame_header: FrameHeader) -> Success {
        let frame: RstStreamFrame = self.expect_payload(frame_header)?;

        Err(Error::connection(
            "stream reset by server",
            frame.payload.error_code.some_box(),
        ))
    }

    fn handle_go_away(&mut self, frame_header: FrameHeader) -> Success {
        let frame: GoAwayFrame = self.expect_payload(frame_header)?;
        let error_message = match frame.payload.additional_debug_data.is_empty() {
            true => "connection reset by server".to_string(),
            false => frame.payload.additional_debug_data.utf8_lossy().to_string(),
        };

        Err(Error::connection(
            &error_message,
            frame.payload.error_code.some_box(),
        ))
    }

    fn handle_misc_frames(&mut self, frame_header: FrameHeader) -> Success {
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
