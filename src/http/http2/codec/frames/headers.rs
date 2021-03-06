use super::Result;
use super::*;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
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
        self.header.flags & Flags::EndHeaders as u8 != 0
    }
}

impl FramePayload for Headers {
    fn parse(bytes: &[u8], flags: u8) -> Result<Self> {
        let mut iter = bytes.into_iter();
        let (pad_length, is_exclusive, stream_dependency, weight) =
            match flag_is_present(Flags::PaddedOrPriority as u8, flags) {
                true => {
                    let pl = match flag_is_present(PADDED, flags) {
                        true => Some(
                            *iter
                                .next()
                                .ok_or_else(|| Error::server("malformed headers frame"))?,
                        ),
                        false => None,
                    };
                    let (ie, sd, w) = match flag_is_present(Flags::Priority as u8, flags) {
                        true => {
                            let mut s = <[u8; 4]>::try_from(
                                iter.by_ref().take(4).map(|b| *b).collect::<Vec<u8>>(),
                            )
                            .map_err(|_| Error::server("malformed headers frame"))?;
                            let e = flag_is_present(Flags::ExclusiveStream as u8, s[0]);
                            let w = iter
                                .next()
                                .ok_or_else(|| Error::server("malformed headers frame"))?;
                            let w = Some(*w);
                            s[0] -= Flags::ExclusiveStream as u8;

                            (Some(e), Some(u32::from_be_bytes(s)), w)
                        }
                        false => (None, None, None),
                    };

                    (pl, ie, sd, w)
                }
                false => (None, None, None, None),
            };
        let (blocks, padding) = parse_blocks_and_pad_length(iter, &pad_length, bytes);

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
                dependency[0] |= Flags::ExclusiveStream as u8;
            }
            bytes.extend(dependency);
        }
        bytes.extend(self.blocks);
        bytes.extend(padding);

        bytes
    }

    fn is_malformed(&self) -> bool {
        false
    }
}

#[repr(u8)]
pub enum Flags {
    EndHeaders = 0x4,
    Priority = 0x20,
    PaddedOrPriority = Flags::EndHeaders as u8 | Flags::Priority as u8,
    ExclusiveStream = 0x80,
}
