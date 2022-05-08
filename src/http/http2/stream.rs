use crate::http;
use crate::http::http2::*;
use crate::http::proto_stream::{Inner, ProtoStream};
use crate::http::{Error, Success};
use std::io::{Read, Write};
use crate::http::utf8::UTF8;

pub struct Http2Stream {
    pub(crate) inner: Inner,
}

impl ProtoStream for Http2Stream {
    const ALPN_PROTOCOLS: Option<&'static [&'static [u8]]> = Some(&[b"h2", b"http/1.1"]);

    fn handshake(&mut self) -> Success {
        self.inner.write_all(crate::http::http2::PREFACE)?;
        if self.inner.conn.alpn_protocol().is_some() {
            self.inner.write_all(SettingsFrame::empty().as_slice());
        }
        let frame: SettingsFrame = self.parse_settings()?;
        for setting in frame.payload {
            println!("{}", setting);
        }

        Ok(())
    }

    fn new(mut stream: Inner) -> Self {
        Self { inner: stream }
    }

    fn inner(&mut self) -> &mut Inner {
        &mut self.inner
    }

    fn empty_buffer() -> Vec<u8> {
        vec![0; Http2Stream::DEFAULT_BUFFER]
    }
}

impl Http2Stream {
    pub const DEFAULT_BUFFER: usize = 8192;

    pub fn parse_frame_header(&mut self) -> http::Result<FrameHeader> {
        let mut buffer = [0; 9];
        self.inner
            .read_exact(&mut buffer)
            .map_err(|e| Error::server("expected frame header"))?;

        Ok(FrameHeader::from(&buffer))
    }

    pub fn parse_payload<P, S>(&mut self, size: S, flags: u8) -> http::Result<P>
    where
        P: FramePayload,
        S: Into<usize>,
    {
        P::parse(self.read_buf(size)?.as_slice(), flags)
    }

    pub fn try_parse_payload<P, S>(&mut self, size: S, flags: u8) -> http::Result<P>
    where
        P: FramePayload,
        S: TryInto<usize>,
    {
        P::parse(self.try_read_buf(size)?.as_slice(), flags)
    }

    pub fn parse_settings(&mut self) -> http::Result<SettingsFrame> {
        let header = self.parse_frame_header()?;
        let payload = self.try_parse_payload(header.length, header.flags)?;

        Ok(Frame::new(header, payload))
    }

    pub fn assess(&mut self) -> (Option<HeadersFrame>, Option<DataFrame>) {
        let mut frame_header = self.parse_frame_header().unwrap();
        if frame_header.kind == SETTING {
            println!("{frame_header:?}");
            println!("Acking");
            println!("Acking : {}", frame_header.flags & 0x01);
            if frame_header.flags & 0x1 == 0 {
                println!("Acking");
                self
                    .inner
                    .write_all(SettingsFrame::ack().as_slice())
                    .unwrap();
            }
            return (None, None);
        }
        if frame_header.kind == 0x06 {
            return (None, None);
        }
        let payload = self.try_read_buf(frame_header.length).unwrap();
        let mut headers: Option<HeadersFrame> = None;
        let mut data: Option<DataFrame> = None;
        let mut window_update_frame: Option<WindowUpdateFrame> = None;
        let mut go_away: Option<GoAwayFrame> = None;
        let mut rst_stream: Option<RstStreamFrame> = None;
        println!("{:?}", frame_header.kind);
        println!("{}", frame_header);
        match frame_header.kind {
            HEADERS => {
                headers = Some(
                    HeadersFrame::parse_from_payload(frame_header, payload.as_slice()).unwrap(),
                )
            }
            DATA => {
                data =
                    Some(DataFrame::parse_from_payload(frame_header, payload.as_slice()).unwrap())
            }
            WINDOW_UPDATE => {
                window_update_frame = Some(
                    WindowUpdateFrame::parse_from_payload(frame_header, payload.as_slice())
                        .unwrap(),
                );
                println!("{window_update_frame:?}");
            }
            RST_STREAM => {
                rst_stream = Some(
                    RstStreamFrame::parse_from_payload(frame_header, payload.as_slice()).unwrap(),
                );
                println!("{rst_stream:?}");
                panic!("stream reset");
            }
            GOAWAY => {
                go_away = Some(
                    GoAwayFrame::parse_from_payload(frame_header, payload.as_slice()).unwrap(),
                );
                println!("{go_away:?}");
                go_away.unwrap().payload.additional_debug_data.print_utf8();
            }
            SETTING => {}
            _ => panic!("whoops"),
        };

        (headers, data)
    }
}
