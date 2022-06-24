use super::*;

pub struct PushPromise {
    pad_length: Option<u8>,
    promised_stream_id: u32,
    header_block_fragment: Vec<u8>,
    padding: Option<Vec<u8>>,
}

impl FramePayload for PushPromise {
    fn parse(bytes: &[u8], flags: u8) -> crate::http::Result<Self> {
        let mut iter = bytes.into_iter();
        let pad_length = match flag_is_present(Flags::Padded as u8, flags) {
            true => Some(
                *iter
                    .next()
                    .ok_or_else(|| Error::server("malformed push_promise frame"))?,
            ),
            false => None,
        };
        let promised_stream_id = u32::from_be_bytes(
            <[u8; 4]>::try_from(iter.by_ref().take(4).map(|b| *b).collect::<Vec<u8>>())
                .map_err(|_| Error::server("received malformed push_promise frame"))?,
        );
        let (header_block_fragment, padding) =
            parse_blocks_and_pad_length(iter, &pad_length, bytes);

        Ok(Self {
            pad_length,
            promised_stream_id,
            header_block_fragment,
            padding,
        })
    }

    fn encode(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.header_block_fragment.len());
        bytes.extend(self.promised_stream_id.to_be_bytes());
        let padding = encode_padding(&mut bytes, self.pad_length, self.padding);
        bytes.extend(self.header_block_fragment);
        bytes.extend(padding);

        bytes
    }

    fn is_malformed(&self) -> bool {
        false
    }
}

#[repr(u8)]
pub enum Flags {
    _EndHeaders = 0x4,
    Padded = 0x8,
}
