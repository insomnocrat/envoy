use super::Result;
use super::*;

const KIND: u8 = 0x01;
pub(crate) const END_HEADERS: u8 = 0x4;
const PRIORITY: u8 = 0x20;
const PADDED_OR_PRIORITY: u8 = PADDED | PRIORITY;
const EXCLUSIVE_STREAM: u8 = 0x80;

#[derive(Clone, Debug)]
pub struct Headers {
    pub pad_length: Option<u8>,
    pub is_exclusive: Option<bool>,
    pub stream_dependency: Option<u32>,
    pub weight: Option<u8>,
    pub blocks: Vec<u8>,
    pub padding: Option<Vec<u8>>,
}

impl Frame<Headers> {
    pub fn is_end_headers(&self) -> bool {
        self.header.flags & END_HEADERS != 0
    }
}

impl FramePayload for Headers {
    fn parse(bytes: &[u8], flags: u8) -> Result<Self> {
        let mut iter = bytes.into_iter();
        let (pad_length, is_exclusive, stream_dependency, weight) =
            match flag_is_present(PADDED_OR_PRIORITY, flags) {
                true => {
                    let pl = match flag_is_present(PADDED, flags) {
                        true => Some(
                            *iter
                                .next()
                                .ok_or_else(|| Error::server("malformed headers frame"))?,
                        ),
                        false => None,
                    };
                    let (ie, sd, w) = match flag_is_present(PRIORITY, flags) {
                        true => {
                            let mut s = <[u8; 4]>::try_from(
                                iter.by_ref().take(4).map(|b| *b).collect::<Vec<u8>>(),
                            )
                            .map_err(|_| Error::server("malformed headers frame"))?;
                            let e = flag_is_present(EXCLUSIVE_STREAM, s[0]);
                            let w = iter
                                .next()
                                .ok_or_else(|| Error::server("malformed headers frame"))?;
                            let w = Some(*w);
                            s[0] -= EXCLUSIVE_STREAM;

                            (Some(e), Some(u32::from_be_bytes(s)), w)
                        }
                        false => (None, None, None),
                    };

                    (pl, ie, sd, w)
                }
                false => (None, None, None, None),
            };

        let (blocks, padding) = match &pad_length {
            None => (iter.map(|b| *b).collect::<Vec<u8>>(), None),
            Some(len) => (
                iter.by_ref()
                    .take(bytes.len() - *len as usize)
                    .map(|b| *b)
                    .collect::<Vec<u8>>(),
                Some(iter.map(|b| *b).collect::<Vec<u8>>()),
            ),
        };

        Ok(Self {
            pad_length,
            is_exclusive,
            stream_dependency,
            weight,
            blocks,
            padding,
        })
    }

    fn encode(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.blocks.len());
        let padding = encode_padding(&mut bytes, self.pad_length, self.padding);
        if let Some(dependency) = self.stream_dependency {
            let mut dependency = dependency.to_be_bytes();
            if let Some(true) = self.is_exclusive {
                dependency[0] |= EXCLUSIVE_STREAM;
            }
            bytes.extend(dependency);
        }
        bytes.extend(self.blocks);
        bytes.extend(padding);

        bytes
    }
}
