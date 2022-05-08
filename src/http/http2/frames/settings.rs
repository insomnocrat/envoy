use crate::http::http2::*;
use crate::http::{Error, Result};
use std::fmt::{Display, Formatter};

const SETTINGS_HEADER_TABLE_SIZE: u16 = 0x01;
const SETTINGS_ENABLE_PUSH: u16 = 0x02;
const SETTINGS_MAX_CONCURRENT_STREAMS: u16 = 0x03;
const SETTINGS_INITIAL_WINDOW_SIZE: u16 = 0x04;
const SETTINGS_MAX_FRAME_SIZE: u16 = 0x05;
const SETTINGS_MAX_HEADER_LIST_SIZE: u16 = 0x06;

const KIND: u8 = 0x04;

pub(crate) type Settings = Vec<Setting>;

impl Frame<Settings> {
    pub fn empty() -> Vec<u8> {
        Self::new(
            FrameHeader {
                length: 0,
                kind: KIND,
                flags: 0,
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
                kind: KIND,
                flags: 0x1,
                stream_identifier: 0,
            },
            Settings::new(),
        )
        .encode()
    }
}

#[derive(Clone, Debug)]
pub struct Setting {
    pub identifier: u16,
    pub value: u32,
}

impl Display for Setting {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.identifier {
            SETTINGS_HEADER_TABLE_SIZE => write!(f, "Header Table Size: {}", &self.value),
            SETTINGS_ENABLE_PUSH => write!(f, "EnablePush: {}", &self.value),
            SETTINGS_INITIAL_WINDOW_SIZE => write!(f, "Initial Window Size: {}", &self.value),
            SETTINGS_MAX_CONCURRENT_STREAMS => write!(f, "Max Concurrent Streams: {}", &self.value),
            SETTINGS_MAX_FRAME_SIZE => write!(f, "Max Frame Size: {}", &self.value),
            SETTINGS_MAX_HEADER_LIST_SIZE => write!(f, "Max Header List Size: {}", &self.value),
            _ => write!(f, "Unknown Setting: {}", &self.value),
        }
    }
}

impl Setting {
    fn encode(self) -> Vec<u8> {
        let mut encoded = self.identifier.to_be_bytes().to_vec();
        encoded.extend(self.value.to_be_bytes());

        encoded
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
}

impl From<[u8; 6]> for Setting {
    fn from(bytes: [u8; 6]) -> Self {
        Self {
            identifier: u16::from_be_bytes([bytes[0], bytes[1]]),
            value: u32::from_be_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
        }
    }
}

impl From<(u16, u32)> for Setting {
    fn from(bytes: (u16, u32)) -> Self {
        let (identifier, value) = bytes;
        Self { identifier, value }
    }
}

impl TryFrom<&[u8]> for Setting {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
        let bytes =
            <[u8; 6]>::try_from(bytes).map_err(|_| Error::server("received malformed setting"))?;

        Ok(Self::from(bytes))
    }
}
