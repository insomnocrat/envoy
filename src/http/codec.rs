use super::{Response, Result};
use crate::http::{Error, Success};
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
    fn decode_status(&self, status: &str) -> Result<u16> {
        u16::from_str(&status).map_err(|_e| Error::server("could not parse status code"))
    }
}
