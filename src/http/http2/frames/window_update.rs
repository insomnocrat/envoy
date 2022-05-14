use super::*;
const KIND: u8 = WINDOW_UPDATE;

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct WindowUpdate {
    pub window_size_increment: u32,
}

impl WindowUpdate {
    pub fn new(size: u32) -> Self {
        Self {
            window_size_increment: size,
        }
    }
    pub(crate) fn to_frame(self) -> Frame<Self> {
        self.into()
    }
}

impl From<WindowUpdate> for Frame<WindowUpdate> {
    fn from(w: WindowUpdate) -> Self {
        Self {
            header: FrameHeader::new(KIND, 0, 0),
            payload: w,
        }
    }
}

impl FramePayload for WindowUpdate {
    fn parse(bytes: &[u8], _flags: u8) -> Result<Self> {
        let bytes = <[u8; 4]>::try_from(bytes).map_err(|_| Error::server("invalid frame"))?;

        Ok(Self {
            window_size_increment: u32::from_be_bytes(bytes),
        })
    }

    fn encode(self) -> Vec<u8> {
        self.window_size_increment.to_be_bytes().to_vec()
    }
}
