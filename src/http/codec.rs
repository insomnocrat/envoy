use super::{Response, Result};
use crate::http::utf8_utils::UTF8Utils;
use crate::http::{Error, Protocol, Success};
use crate::rest::response::RequestBuilder;
use rustls::ClientConnection as TlsClient;
use rustls::StreamOwned as TlsStream;
use std::net::TcpStream;
use std::str::FromStr;

pub trait Codec: Send {
    fn encode_request(&mut self, request: RequestBuilder) -> Result<Vec<u8>>;
    fn decode_response(&mut self, conn: &mut TlsStream<TlsClient, TcpStream>) -> Result<Response>;
    fn empty_buffer(&self) -> Vec<u8>;
    fn handshake(&mut self, conn: &mut TlsStream<TlsClient, TcpStream>) -> Success;
    fn decode_status(&self, status: &[u8]) -> Result<u16> {
        u16::from_str(&status.as_utf8_lossy())
            .map_err(|_e| Error::server("could not parse status code"))
    }
    fn kind(&self) -> Protocol;
}
