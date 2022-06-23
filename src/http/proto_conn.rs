use super::{error::SomeError, Error, Result};
use crate::http::codec::Codec;
use crate::http::http1::codec::Http1Codec;
#[cfg(feature = "http2")]
use crate::http::http2::codec::Http2Codec;
use crate::http::request::RequestBuilder;
use crate::http::{Protocol, Response, Success};
use rustls::client::InvalidDnsNameError;
use rustls::ClientConnection as TlsClient;
use rustls::StreamOwned as TlsStream;
use std::io::Write;
use std::net::TcpStream;
use std::sync::Arc;

pub(crate) type Inner = TlsStream<TlsClient, TcpStream>;

#[cfg(feature = "http2")]
pub const H2: &[u8] = b"h2";
pub const H1: &[u8] = b"http/1.1";
pub const ALPN: &[&[u8]] = &[
    #[cfg(feature = "http2")]
    H2,
    H1,
];

pub struct ProtoConn {
    pub(crate) inner: Inner,
    pub(crate) codec: Box<dyn Codec>,
    pub(crate) authority: String,
}

impl ProtoConn {
    pub fn new(authority: &str, protocol: Protocol) -> Result<Self> {
        let stream = TcpStream::connect(authority)?;
        let tls_client = Self::config_tls(
            authority.trim_end_matches(|c: char| c == ':' || c.is_numeric()),
            ALPN,
        )?;
        let mut conn = match protocol {
            Protocol::HTTP1 => Self {
                inner: TlsStream::new(tls_client, stream),
                codec: Box::new(Http1Codec::new()),
                authority: authority.to_string(),
            },
            #[cfg(feature = "http2")]
            Protocol::HTTP2 => Self {
                inner: TlsStream::new(tls_client, stream),
                codec: Box::new(Http2Codec::new()),
                authority: authority.to_string(),
            },
        };
        conn.codec.prelude(&mut conn.inner)?;

        Ok(conn)
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

    #[cfg(feature = "http2")]
    pub fn downgrade_protocol(&mut self) -> Success {
        *self = Self::new(&self.authority, Protocol::HTTP1)?;

        Ok(())
    }

    pub fn reset(&mut self) -> Success {
        *self = Self::new(&self.authority, self.codec.kind())?;

        Ok(())
    }

    pub fn send_request(&mut self, request: RequestBuilder) -> Result<Response> {
        let encoded = self.codec.encode_request(request)?;
        self.inner.write_all(&encoded)?;
        self.inner.flush()?;
        self.codec.decode_response(&mut self.inner)
    }
}
