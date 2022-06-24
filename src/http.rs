use std::collections::HashMap;
use std::fmt::{Display, Formatter};

pub mod client;
mod codec;
pub mod error;
pub mod http1;
#[cfg(feature = "http2")]
pub mod http2;
pub mod pool;
pub mod pooled_conn;
mod proto_conn;
pub mod request;
pub mod status;
#[cfg(test)]
pub(crate) mod test_utils;
pub mod url;
pub mod utf8_utils;

use crate::http::error::ErrorKind;
use crate::http::utf8_utils::UTF8Utils;
pub use error::Error;

type Result<T> = std::result::Result<T, Error>;
type Success = Result<()>;
pub type HttpClient = client::Client;

#[derive(Debug, Clone, Copy)]
pub enum Method {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    CONNECT,
}

#[derive(Debug, Clone, Eq, PartialEq, Copy)]
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

impl Display for Protocol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let p = match &self {
            Self::HTTP1 => "HTTP/1.1",
            #[cfg(feature = "http2")]
            Self::HTTP2 => "HTTP/2",
        };
        write!(f, "{p}")
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
                &format!("invalid http protocol {}", bytes.as_utf8_lossy()),
                ErrorKind::Server,
            )),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Response {
    pub protocol: Protocol,
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Response {{\nprotocol: {},\nstatus_code: {},\nheaders: {:#?},\nbody: {}\n}}",
            self.protocol,
            self.status_code,
            self.headers,
            self.body.as_utf8_lossy()
        )
    }
}
