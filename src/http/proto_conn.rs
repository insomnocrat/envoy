use super::{error::SomeError, Error, Result};
use crate::http::codec::Codec;
use crate::http::http1::codec::Http1Codec;
#[cfg(feature = "http2")]
use crate::http::http2::codec::Http2Codec;
use crate::http::request::RequestBuilder;
#[cfg(feature = "http2")]
use crate::http::Protocol;
use crate::http::Response;
use rustls::client::InvalidDnsNameError;
use rustls::ClientConnection as TlsClient;
use rustls::StreamOwned as TlsStream;
use std::io::Write;
use std::net::TcpStream;
use std::sync::Arc;

pub(crate) type Inner = TlsStream<TlsClient, TcpStream>;

#[cfg(feature = "http2")]
pub const H2: &[u8] = b"h2";
#[cfg(feature = "http2")]
pub const H1: &[u8] = b"http/1.1";
pub const ALPN: &[&[u8]] = &[
    #[cfg(feature = "http2")]
    H2,
    #[cfg(feature = "http2")]
    H1,
];

pub struct ProtoConn {
    pub(crate) inner: Inner,
    pub(crate) codec: Box<dyn Codec>,
}

impl ProtoConn {
    pub(crate) fn connect(authority: &str) -> Result<Self> {
        let stream = TcpStream::connect(authority)?;
        let tls_client = Self::config_tls(
            authority.trim_end_matches(|c: char| c == ':' || c.is_numeric()),
            ALPN,
        )?;
        let inner_stream: Inner;
        let codec: Box<dyn Codec>;
        #[cfg(feature = "http2")]
        {
            let mut stream = TlsStream::new(tls_client, stream);
            let mut c = Box::new(Http2Codec::new());
            match c.handshake(&mut stream) {
                Ok(_) => codec = c,
                Err(_) => codec = Box::new(Http1Codec::new()),
            }
            inner_stream = stream;
        }
        #[cfg(not(feature = "http2"))]
        {
            codec = Box::new(Http1Codec::new());
            inner_stream = TlsStream::new(tls_client, stream);
        }

        Ok(Self::new(inner_stream, codec))
    }

    #[cfg(feature = "http2")]
    pub(crate) fn switch_protocol(&mut self, protocol: &Protocol) {
        match protocol {
            Protocol::HTTP1 => self.codec = Box::new(Http1Codec::new()),
            Protocol::HTTP2 => self.codec = Box::new(Http2Codec::new()),
        }
    }

    fn new(stream: Inner, codec: Box<dyn Codec>) -> Self {
        Self {
            inner: stream,
            codec,
        }
    }

    fn config_tls(host: &str, protocols: &[&[u8]]) -> Result<TlsClient> {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));
        let mut config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        protocols
            .into_iter()
            .for_each(|p| config.alpn_protocols.push(p.to_vec()));
        let rc_config = Arc::new(config);

        TlsClient::new(
            rc_config,
            host.try_into().map_err(|e: InvalidDnsNameError| {
                Error::connection("invalid host address", e.to_string().some_box())
            })?,
        )
        .map_err(|e| Error::connection("could not connect to server", e.some_box()))
    }

    pub fn send_request(&mut self, request: RequestBuilder) -> Result<Response> {
        #[cfg(feature = "http2")]
        if request.protocol != self.codec.kind() {
            self.switch_protocol(&request.protocol);
        }
        let encoded = self.codec.encode_request(request)?;
        self.inner.write_all(&encoded)?;

        self.codec.decode_response(&mut self.inner)
    }
}
