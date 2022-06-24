use super::*;

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct Priority {
    pub is_exclusive: bool,
    pub stream_dependency: u32,
    pub weight: u8,
}

impl FramePayload for Priority {
    fn parse(bytes: &[u8], _flags: u8) -> Result<Self> {
        let mut bytes =
            <[u8; 5]>::try_from(bytes).map_err(|_| Error::server("received malformed frame"))?;
        let is_exclusive = flag_is_present(0x1, bytes[0]);
        if is_exclusive {
            bytes[0] -= 0x1;
        }
        let stream_dependency = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let weight = bytes[4];

        Ok(Self {
            is_exclusive,
            stream_dependency,
            weight,
        })
    }

    fn encode(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(5);
        let mut stream_dependency = self.stream_dependency.to_be_bytes();
        if self.is_exclusive {
            stream_dependency[0] |= 0x1;
        }
        bytes.extend(stream_dependency);
        bytes.push(self.weight);

        bytes
    }

    fn is_malformed(&self) -> bool {
        todo!()
    }
}
