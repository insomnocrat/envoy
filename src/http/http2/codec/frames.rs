pub(crate) mod continuation;
pub(crate) mod data;
pub(crate) mod go_away;
pub(crate) mod headers;
pub(crate) mod ping;
pub(crate) mod rst_stream;
pub(crate) mod settings;
pub(crate) mod window_update;
pub(crate) mod push_promise;

pub type SettingsFrame = Frame<settings::Settings>;
pub type HeadersFrame = Frame<headers::Headers>;
pub type DataFrame = Frame<data::Data>;
pub type WindowUpdateFrame = Frame<window_update::WindowUpdate>;
pub type GoAwayFrame = Frame<go_away::GoAway>;
pub type RstStreamFrame = Frame<rst_stream::RstStream>;
pub type ContinuationFrame = Frame<continuation::Continuation>;
pub type PingFrame = Frame<ping::Ping>;

use crate::http::{Error, Result};
use std::fmt::{Debug, Display, Formatter};
use std::slice::Iter;

pub const END_STREAM: u8 = 0x1;
pub const PADDED: u8 = 0x08;
pub const RESERVED: u8 = 0x80;

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
#[repr(u8)]
pub enum FrameKind {
    Data = 0x0,
    Headers = 0x1,
    Priority = 0x2,
    RstStream = 0x3,
    Setting = 0x4,
    PushPromise = 0x5,
    Ping = 0x6,
    GoAway = 0x7,
    WindowUpdate = 0x8,
    Continuation = 0x9,
    Altsvc = 0xa,
    Origin = 0xc,
}

impl TryFrom<u8> for FrameKind {
    type Error = Error;
    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        let result = match value {
            0x0 => Self::Data,
            0x1 => Self::Headers,
            0x2 => Self::Priority,
            0x3 => Self::RstStream,
            0x4 => Self::Setting,
            0x5 => Self::PushPromise,
            0x6 => Self::Ping,
            0x7 => Self::GoAway,
            0x8 => Self::WindowUpdate,
            0x9 => Self::Continuation,
            0xa => Self::Altsvc,
            0xc => Self::Origin,
            _ => return Err(Error::server("received invalid frame")),
        };
        Ok(result)
    }
}

impl TryFrom<&[u8; 9]> for FrameKind {
    type Error = Error;

    fn try_from(value: &[u8; 9]) -> std::result::Result<Self, Self::Error> {
        value[3]
            .try_into()
            .map_err(|_| Error::server("received invalid frame"))
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
#[repr(u32)]
#[non_exhaustive]
pub enum ErrorCode {
    NoError = 0x0,
    ProtocolError = 0x1,
    InternalError = 0x2,
    FlowControlError = 0x3,
    SettingsTimeout = 0x4,
    StreamClosed = 0x5,
    FrameSizeError = 0x6,
    RefusedStream = 0x7,
    Cancel = 0x8,
    CompressionError = 0x9,
    ConnectError = 0xa,
    EnhanceYourCalm = 0xb,
    InadequateSecurity = 0xc,
    Http11Required = 0xd,
    Custom,
}

impl ErrorCode {
    pub fn to_be_bytes(self) -> [u8; 4] {
        (self as u32).to_be_bytes()
    }
}

impl From<u32> for ErrorCode {
    fn from(value: u32) -> Self {
        match value {
            0x0 => Self::NoError,
            0x1 => Self::ProtocolError,
            0x2 => Self::InternalError,
            0x3 => Self::FlowControlError,
            0x4 => Self::SettingsTimeout,
            0x5 => Self::StreamClosed,
            0x6 => Self::FrameSizeError,
            0x7 => Self::RefusedStream,
            0x8 => Self::Cancel,
            0x9 => Self::CompressionError,
            0xa => Self::ConnectError,
            0xb => Self::EnhanceYourCalm,
            0xc => Self::InadequateSecurity,
            0xd => Self::Http11Required,
            _ => Self::Custom,
        }
    }
}

impl From<[u8; 4]> for ErrorCode {
    fn from(bytes: [u8; 4]) -> Self {
        Self::from(u32::from_be_bytes(bytes))
    }
}

pub(crate) const PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

pub trait FramePayload: Sized {
    fn parse(bytes: &[u8], flags: u8) -> Result<Self>;
    fn encode(self) -> Vec<u8>;
    fn is_malformed(&self) -> bool;
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
    pub fn is_stream_end(&self) -> bool {
        self.header.flags & END_STREAM != 0
    }
    pub fn id(&self) -> &u32 {
        &self.header.stream_identifier
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct FrameHeader {
    pub length: u32,
    pub kind: FrameKind,
    pub flags: u8,
    pub stream_identifier: u32,
}

impl Display for FrameHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let kind = match self.kind {
            FrameKind::Data => "Data",
            FrameKind::Headers => "Header",
            FrameKind::Setting => "Setting",
            FrameKind::WindowUpdate => "Window Update",
            FrameKind::GoAway => "Go Away",
            FrameKind::RstStream => "Reset Stream",
            FrameKind::PushPromise => "Push Promise",
            FrameKind::Continuation => "Continuation",
            FrameKind::Altsvc => "Altsvc",
            FrameKind::Origin => "Origin",
            FrameKind::Ping => "Ping",
            FrameKind::Priority => "Priority",
        };
        write!(
            f,
            "FrameHeader {{ length: {}, kind: {kind}, flags: {}, stream_identifier: {} }}",
            self.length, self.flags, self.stream_identifier
        )
    }
}

impl FrameHeader {
    pub fn new(kind: FrameKind, flags: u8, stream_identifier: u32) -> Self {
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
        bytes.push(self.kind as u8);
        bytes.push(self.flags);
        bytes.extend(self.stream_identifier.to_be_bytes());

        bytes
    }
    pub fn is_malformed(&self) -> bool {
        match self.kind {
            FrameKind::Data
            | FrameKind::Headers
            | FrameKind::Continuation
            | FrameKind::Priority
            | FrameKind::RstStream
            | FrameKind::PushPromise => self.stream_identifier == 0,
            FrameKind::Setting | FrameKind::GoAway | FrameKind::Ping => self.stream_identifier != 0,
            _ => false,
        }
    }
}

impl From<&[u8; 9]> for FrameHeader {
    fn from(bytes: &[u8; 9]) -> Self {
        Self {
            length: u32::from_be_bytes([0x00, bytes[0], bytes[1], bytes[2]]),
            kind: bytes[3].try_into().unwrap(),
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
        let kind = bytes.try_into()?;

        Ok(Self {
            length: u32::from_be_bytes([0x00, bytes[0], bytes[1], bytes[2]]),
            kind,
            flags: bytes[4],
            stream_identifier: u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]),
        })
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

fn parse_blocks_and_pad_length(mut iter: Iter<u8>, pad_length: &Option<u8>, bytes: &[u8]) -> (Vec<u8>, Option<Vec<u8>>) {
    match &pad_length {
        None => (iter.map(|b| *b).collect::<Vec<u8>>(), None),
        Some(len) => (
            iter.by_ref()
                .take(bytes.len() - *len as usize)
                .map(|b| *b)
                .collect::<Vec<u8>>(),
            Some(iter.map(|b| *b).collect::<Vec<u8>>()),
        ),
    }
}
