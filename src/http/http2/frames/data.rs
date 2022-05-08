use super::Result;
use super::*;

#[derive(Clone, Debug)]
pub struct Data {
    pub pad_length: Option<u8>,
    pub blocks: Vec<u8>,
    pub padding: Option<Vec<u8>>,
}

impl FramePayload for Data {
    fn parse(bytes: &[u8], flags: u8) -> Result<Self> {
        let (pad_length, blocks, padding) = match flag_is_present(PADDED, flags) {
            true => {
                let pl = bytes[0];
                let padded = (bytes.len() - pl as usize);
                (
                    Some(pl),
                    bytes[1..padded].to_vec(),
                    Some(bytes[padded..].to_vec()),
                )
            }
            false => (None, bytes.to_vec(), None),
        };

        Ok(Self {
            pad_length,
            blocks,
            padding,
        })
    }

    fn encode(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.blocks.len());
        let padding = encode_padding(&mut bytes, self.pad_length, self.padding);
        bytes.extend(self.blocks);
        bytes.extend(padding);

        bytes
    }
}
