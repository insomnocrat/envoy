use super::*;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct GoAway {
    pub last_stream_id: u32,
    pub error_code: ErrorCode,
    pub additional_debug_data: Vec<u8>,
}

impl GoAway {
    pub fn new(error_code: ErrorCode, additional_debug_data: Option<Vec<u8>>) -> Self {
        let additional_debug_data = match additional_debug_data {
            Some(i) => i,
            None => Vec::new(),
        };

        Self {
            last_stream_id: 0,
            error_code,
            additional_debug_data,
        }
    }
    pub fn to_frame(self) -> Frame<Self> {
        Frame::new(FrameHeader::new(FrameKind::GoAway, 0, 0), self)
    }
}

impl FramePayload for GoAway {
    fn parse(bytes: &[u8], _flags: u8) -> Result<Self> {
        let additional_debug_data = match bytes.len() > 8 {
            true => bytes[8..].to_vec(),
            false => Vec::new(),
        };

        Ok(Self {
            last_stream_id: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            error_code: ErrorCode::from([bytes[4], bytes[5], bytes[6], bytes[7]]),
            additional_debug_data,
        })
    }

    fn encode(self) -> Vec<u8> {
        let mut bytes = self.last_stream_id.to_be_bytes().to_vec();
        bytes.extend(self.error_code.to_be_bytes());
        bytes.extend(self.additional_debug_data);

        bytes
    }

    fn is_malformed(&self) -> bool {
        false
    }
}
