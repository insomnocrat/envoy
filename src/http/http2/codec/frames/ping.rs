use super::*;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Copy)]
pub struct Ping {
    opaque_data: u64,
}

impl Ping {
    pub(crate) fn new(opaque_data: u64) -> Self {
        Self { opaque_data }
    }

    pub(crate) fn to_frame(self) -> Frame<Self> {
        Frame::new(FrameHeader::new(FrameKind::Ping, 0, 0), self)
    }
}

impl FramePayload for Ping {
    fn parse(bytes: &[u8], _flags: u8) -> crate::http::Result<Self> {
        let bytes =
            <[u8; 8]>::try_from(bytes).map_err(|_| Error::server("received malformed payload"))?;
        let opaque_data = u64::from_be_bytes(bytes);

        Ok(Self { opaque_data })
    }

    fn encode(self) -> Vec<u8> {
        self.opaque_data.to_be_bytes().to_vec()
    }

    fn is_malformed(&self) -> bool {
        false
    }
}

#[repr(u8)]
pub enum Flags {
    Ack = 0x1,
}
