use std::collections::HashMap;

pub mod buffer;
pub mod client;
pub mod connection;
pub mod error;
pub mod pool;
pub mod request;
#[cfg(test)]
mod tests;
pub mod utf8;

use crate::http::error::ErrorKind;
use crate::http::utf8::UTF8;
pub use error::Error;

type Result<T> = std::result::Result<T, Error>;
type Success = Result<()>;

#[derive(Debug, Clone)]
pub enum Method {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

#[derive(Debug, Clone)]
pub enum Protocol {
    HTTP1,
    #[cfg(feature = "http2")]
    HTTP2,
}

impl Default for Protocol {
    #[cfg(feature = "http2")]
    fn default() -> Self {
        Self::HTTP2
    }
    #[cfg(not(feature = "http2"))]
    fn default() -> Self {
        Self::HTTP1
    }
}

impl<'a> TryFrom<&[u8]> for Protocol {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        match bytes {
            b"HTTP/1.1" => Ok(Self::HTTP1),
            #[cfg(feature = "http2")]
            b"HTTP/2.0" => Ok(Self::HTTP2),
            _ => Err(Error::new(
                &format!("invalid http protocol {}", bytes.utf8_lossy()),
                ErrorKind::Server,
            )),
        }
    }
}

#[derive(Debug)]
pub struct Response {
    pub protocol: Protocol,
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}
