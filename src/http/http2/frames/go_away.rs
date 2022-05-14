use super::*;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct GoAway {
    pub last_stream_id: u32,
    pub error_code: u32,
    pub additional_debug_data: Vec<u8>,
}

impl FramePayload for GoAway {
    fn parse(bytes: &[u8], _flags: u8) -> Result<Self> {
        let additional_debug_data = match bytes.len() > 8 {
            true => bytes[8..].to_vec(),
            false => Vec::new(),
        };
        Ok(Self {
            last_stream_id: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            error_code: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            additional_debug_data,
        })
    }

    fn encode(self) -> Vec<u8> {
        let mut bytes = self.last_stream_id.to_be_bytes().to_vec();
        bytes.extend(self.error_code.to_be_bytes());
        bytes.extend(self.additional_debug_data);

        bytes
    }
}
