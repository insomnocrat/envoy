use crate::http::http2::codec::StreamSettings;
use crate::http::http2::*;
use crate::http::{Error, Result};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
#[repr(u16)]
pub enum Identifier {
    HeaderTableSize = 0x1,
    EnablePush = 0x2,
    MaxConcurrentStreams = 0x3,
    InitialWindowSize = 0x4,
    MaxFrameSize = 0x5,
    MaxHeaderListSize = 0x6,
}

#[repr(u8)]
pub enum Flags {
    Ack = 0x1,
}

impl Identifier {
    pub fn to_be_bytes(self) -> [u8; 2] {
        (self as u16).to_be_bytes()
    }
}

impl TryFrom<u16> for Identifier {
    type Error = Error;

    fn try_from(value: u16) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            0x1 => Self::HeaderTableSize,
            0x2 => Self::EnablePush,
            0x3 => Self::MaxConcurrentStreams,
            0x4 => Self::InitialWindowSize,
            0x5 => Self::MaxFrameSize,
            0x6 => Self::MaxHeaderListSize,
            _ => return Err(Error::server("received invalid settings frame")),
        })
    }
}

pub(crate) type Settings = Vec<Setting>;

impl Frame<Settings> {
    pub fn empty() -> Vec<u8> {
        Self::new(
            FrameHeader {
                length: 0,
                kind: FrameKind::Setting,
                flags: 0x0,
                stream_identifier: 0,
            },
            Settings::new(),
        )
        .encode()
    }
    pub fn ack() -> Vec<u8> {
        Self::new(
            FrameHeader {
                length: 0,
                kind: FrameKind::Setting,
                flags: Flags::Ack as u8,
                stream_identifier: 0,
            },
            Settings::new(),
        )
        .encode()
    }
}

#[derive(Clone, Debug)]
pub struct Setting {
    pub identifier: Identifier,
    pub value: u32,
}

impl Display for Setting {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.identifier {
            Identifier::HeaderTableSize => write!(f, "Header Table Size: {}", &self.value),
            Identifier::EnablePush => write!(f, "EnablePush: {}", &self.value),
            Identifier::InitialWindowSize => write!(f, "Initial Window Size: {}", &self.value),
            Identifier::MaxConcurrentStreams => {
                write!(f, "Max Concurrent Streams: {}", &self.value)
            }
            Identifier::MaxFrameSize => write!(f, "Max Frame Size: {}", &self.value),
            Identifier::MaxHeaderListSize => write!(f, "Max Header List Size: {}", &self.value),
        }
    }
}

impl Setting {
    fn encode(self) -> Vec<u8> {
        let mut encoded = (self.identifier as u16).to_be_bytes().to_vec();
        encoded.extend(self.value.to_be_bytes());

        encoded
    }
    fn is_malformed(&self) -> bool {
        match &self.identifier {
            Identifier::InitialWindowSize => self.value > 2147483647,
            Identifier::MaxFrameSize => self.value > 16777215,
            _ => false,
        }
    }
}

impl FramePayload for Settings {
    fn parse(bytes: &[u8], _flags: u8) -> Result<Self> {
        let mut settings = Vec::with_capacity(6);
        let bytes = bytes.chunks_exact(6);
        for byte in bytes {
            settings.push(Setting::try_from(byte)?);
        }

        Ok(settings)
    }
    fn encode(self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for setting in self {
            bytes.extend(setting.encode());
        }

        bytes
    }

    fn is_malformed(&self) -> bool {
        for setting in self.iter() {
            if setting.is_malformed() {
                return true;
            }
        }

        false
    }
}

impl TryFrom<[u8; 6]> for Setting {
    type Error = Error;

    fn try_from(bytes: [u8; 6]) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            identifier: u16::from_be_bytes([bytes[0], bytes[1]]).try_into()?,
            value: u32::from_be_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
        })
    }
}

impl TryFrom<(u16, u32)> for Setting {
    type Error = Error;

    fn try_from(bytes: (u16, u32)) -> std::result::Result<Self, Self::Error> {
        let (identifier, value) = bytes;
        Ok(Self {
            identifier: identifier.try_into()?,
            value,
        })
    }
}

impl TryFrom<&[u8]> for Setting {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
        <[u8; 6]>::try_from(bytes)
            .map_err(|_| Error::server("received malformed setting frame"))?
            .try_into()
    }
}

impl From<&StreamSettings> for Settings {
    fn from(s: &StreamSettings) -> Self {
        let header_table_size = Setting {
            identifier: Identifier::HeaderTableSize,
            value: s.header_table_size,
        };
        let enable_push = match s.enable_push {
            true => Setting {
                identifier: Identifier::EnablePush,
                value: 1,
            },
            false => Setting {
                identifier: Identifier::EnablePush,
                value: 0,
            },
        };
        let max_concurrent_streams = Setting {
            identifier: Identifier::MaxConcurrentStreams,
            value: s.max_concurrent_streams,
        };
        let initial_window_size = Setting {
            identifier: Identifier::InitialWindowSize,
            value: s.initial_window_size,
        };
        let max_frame_size = Setting {
            identifier: Identifier::MaxFrameSize,
            value: s.max_frame_size,
        };
        let max_header_list = Setting {
            identifier: Identifier::MaxHeaderListSize,
            value: s.max_header_list_size,
        };

        vec![
            header_table_size,
            enable_push,
            max_concurrent_streams,
            initial_window_size,
            max_frame_size,
            max_header_list,
        ]
    }
}
