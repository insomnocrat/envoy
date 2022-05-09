use super::*;

#[derive(Clone, Debug)]
pub struct WindowUpdate {
    pub window_size_increment: u32,
}

impl FramePayload for WindowUpdate {
    fn parse(bytes: &[u8], _flags: u8) -> Result<Self> {
        let mut bytes = <[u8; 4]>::try_from(bytes).map_err(|_| Error::server("invalid frame"))?;
        bytes[0] &= RESERVED;
        Ok(Self {
            window_size_increment: u32::from_be_bytes(bytes),
        })
    }

    fn encode(self) -> Vec<u8> {
        self.window_size_increment.to_be_bytes().to_vec()
    }
}
