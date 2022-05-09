pub(crate) mod data;
pub(crate) mod go_away;
pub(crate) mod headers;
pub(crate) mod rst_stream;
pub(crate) mod settings;
pub(crate) mod window_update;

pub type SettingsFrame = Frame<settings::Settings>;
pub type HeadersFrame = Frame<headers::Headers>;
pub type DataFrame = Frame<data::Data>;
pub type WindowUpdateFrame = Frame<window_update::WindowUpdate>;
pub type GoAwayFrame = Frame<go_away::GoAway>;
pub type RstStreamFrame = Frame<rst_stream::RstStream>;

use crate::http::{Error, Result};
use std::fmt::{Display, Formatter};

pub const DATA: u8 = 0x00;
pub const HEADERS: u8 = 0x01;
pub const PRIORITY: u8 = 0x02;
pub const RST_STREAM: u8 = 0x03;
pub const SETTING: u8 = 0x04;
pub const PUSH_PROMISE: u8 = 0x05;
pub const PING: u8 = 0x06;
pub const GOAWAY: u8 = 0x07;
pub const WINDOW_UPDATE: u8 = 0x08;
pub const CONTINUATION: u8 = 0x09;
pub const ALTSVC: u8 = 0x0a;
pub const ORIGIN: u8 = 0x0c;

pub const END_STREAM: u8 = 0x1;
pub const PADDED: u8 = 0x08;
pub const RESERVED: u8 = 0x80;

pub(crate) const PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

pub trait FramePayload: Sized {
    fn parse(bytes: &[u8], flags: u8) -> Result<Self>;
    fn encode(self) -> Vec<u8>;
}

#[derive(Clone, Debug)]
pub struct Frame<P>
where
    P: FramePayload,
{
    pub header: FrameHeader,
    pub payload: P,
}

impl<P: FramePayload> Frame<P> {
    pub(crate) fn new(header: FrameHeader, payload: P) -> Self {
        Self { header, payload }
    }
    pub fn encode(mut self) -> Vec<u8> {
        let payload = self.payload.encode();
        self.header.length = payload.len() as u32;
        let mut bytes = self.header.to_bytes();
        bytes.extend(payload);

        bytes
    }
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let header = FrameHeader::try_from(bytes)?;
        let payload = P::parse(&bytes[9..(9 + header.length as usize)], header.flags)?;

        Ok(Self::new(header, payload))
    }
    pub fn parse_from_payload(header: FrameHeader, bytes: &[u8]) -> Result<Self> {
        let payload = P::parse(bytes, header.flags)?;

        Ok(Self::new(header, payload))
    }
}

#[derive(Clone, Debug)]
pub struct FrameHeader {
    pub length: u32,
    pub kind: u8,
    pub flags: u8,
    pub stream_identifier: u32,
}

impl Display for FrameHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let kind = match self.kind {
            DATA => "Data",
            HEADERS => "Header",
            SETTING => "Setting",
            WINDOW_UPDATE => "Window Update",
            GOAWAY => "Go Away",
            RST_STREAM => "Reset Stream",
            PUSH_PROMISE => "Push Promise",
            CONTINUATION => "Continuation",
            ALTSVC => "Altsvc",
            ORIGIN => "Origin",
            PING => "Ping",
            PRIORITY => "Priority",
            _ => "Unknown",
        };
        write!(
            f,
            "FrameHeader {{ length: {}, kind: {kind}, flags: {}, stream_identifier: {} }}",
            self.length, self.flags, self.stream_identifier
        )
    }
}

impl FrameHeader {
    pub fn new(kind: u8, flags: u8, stream_identifier: u32) -> Self {
        Self {
            length: 0,
            kind,
            flags,
            stream_identifier,
        }
    }
    pub fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let length = self.length.to_be_bytes();
        let length = match length.len() {
            4 => &length[1..],
            _ => &length,
        };
        bytes.extend(length);
        bytes.push(self.kind);
        bytes.push(self.flags);
        bytes.extend(self.stream_identifier.to_be_bytes());

        bytes
    }
}

impl From<&[u8; 9]> for FrameHeader {
    fn from(bytes: &[u8; 9]) -> Self {
        Self {
            length: u32::from_be_bytes([0x00, bytes[0], bytes[1], bytes[2]]),
            kind: bytes[3],
            flags: bytes[4],
            stream_identifier: u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]),
        }
    }
}

impl TryFrom<&[u8]> for FrameHeader {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
        let bytes = <&[u8; 9]>::try_from(&bytes[0..9])
            .map_err(|_| Error::server("invalid frame header"))?;

        Ok(Self::from(bytes))
    }
}

fn flag_is_present(flag: u8, delivered: u8) -> bool {
    delivered & flag != 0x00
}

fn encode_padding(
    bytes: &mut Vec<u8>,
    pad_length: Option<u8>,
    maybe_padding: Option<Vec<u8>>,
) -> Vec<u8> {
    let padding: Vec<u8>;
    match (pad_length, maybe_padding) {
        (Some(pad_length), Some(padded)) => {
            bytes.push(pad_length);
            padding = padded;
        }
        (None, Some(padded)) => {
            bytes.push(padded.len() as u8);
            padding = padded
        }
        (_, _) => padding = Vec::new(),
    }

    padding
}
