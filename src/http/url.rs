use crate::http::utf8_utils::{UTF8Utils, QMARK, SLASH};
use std::fmt::{Display, Formatter};

pub const SCHEME: &[u8] = b"https://";
pub const AUTHORITY: &[u8] = b"www.";

#[derive(Clone, Debug)]
pub struct Url {
    pub host: Vec<u8>,
    pub resource: Vec<u8>,
    pub query: Vec<u8>,
}

impl Display for Url {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.host.as_utf8_lossy(),
            self.resource.as_utf8_lossy(),
            self.query.as_utf8_lossy()
        )
    }
}

impl Url {
    pub fn authority(&self) -> String {
        format!("{}:443", self.host.as_utf8_lossy())
    }
}

impl<T: AsRef<[u8]>> From<T> for Url {
    fn from(value: T) -> Self {
        let mut value = value.as_ref();
        if value.starts_with(SCHEME) {
            value = &value[7..];
        }
        if value.starts_with(AUTHORITY) {
            value = &value[3..];
        }
        let mut host = Vec::with_capacity(2048);
        let mut resource = Vec::new();
        let mut query = Vec::new();
        let mut value = value.iter().peekable();
        while let Some(byte) = value.next_if(|b| **b != SLASH) {
            host.push(*byte);
        }
        while let Some(byte) = value.next_if(|b| **b != QMARK) {
            resource.push(*byte);
        }
        query.extend(value);

        Self {
            host,
            resource,
            query,
        }
    }
}
