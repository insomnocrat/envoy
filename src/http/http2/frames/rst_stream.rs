use super::*;

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct RstStream {
    pub error_code: ErrorCode,
}

impl FramePayload for RstStream {
    fn parse(bytes: &[u8], _flags: u8) -> Result<Self> {
        let bytes = <[u8; 4]>::try_from(bytes).map_err(|_| Error::server("invalid frame"))?;

        Ok(Self {
            error_code: ErrorCode::from(bytes),
        })
    }

    fn encode(self) -> Vec<u8> {
        self.error_code.to_be_bytes().to_vec()
    }
}
