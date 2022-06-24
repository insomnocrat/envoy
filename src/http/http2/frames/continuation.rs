use super::Result;
use super::*;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Continuation {
    pub blocks: Vec<u8>,
}

impl Frame<Continuation> {
    pub fn is_end_headers(&self) -> bool {
        self.header.flags & Flags::EndHeaders as u8 != 0
    }
}

impl FramePayload for Continuation {
    fn parse(bytes: &[u8], _flags: u8) -> Result<Self> {
        Ok(Self {
            blocks: bytes.to_vec(),
        })
    }

    fn encode(self) -> Vec<u8> {
        self.blocks
    }

    fn is_malformed(&self) -> bool {
        false
    }
}

#[repr(u8)]
pub enum Flags {
    EndHeaders = 0x4,
}
