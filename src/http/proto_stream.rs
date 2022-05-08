use super::{error::SomeError, Error, Result, Success};
use crate::http::request::RequestBuilder;
use crate::http::Response;
use rustls::client::InvalidDnsNameError;
use rustls::ClientConnection as TlsClient;
use rustls::StreamOwned as TlsStream;
use std::io::Read;
use std::net::TcpStream;
use std::sync::Arc;

pub(crate) type Inner = TlsStream<TlsClient, TcpStream>;

pub trait ProtoStream: Sized + Send {
    const ALPN_PROTOCOLS: Option<&'static [&'static [u8]]> = None;

    fn connect(authority: &str) -> Result<Self> {
        let stream = TcpStream::connect(authority)?;
        let tls_client =
            Self::config_tls(authority.trim_end_matches(|c: char| c == ':' || c.is_numeric()))?;
        let stream = TlsStream::new(tls_client, stream);
        let mut proto_stream = Self::new(stream);
        proto_stream.handshake()?;

        Ok(proto_stream)
    }

    fn handshake(&mut self) -> Success;

    fn new(stream: Inner) -> Self;

    fn config_tls(host: &str) -> Result<TlsClient> {
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
        if let Some(protocols) = Self::ALPN_PROTOCOLS {
            protocols
                .iter()
                .for_each(|p| config.alpn_protocols.push(p.to_vec()))
        }
        let rc_config = Arc::new(config);

        TlsClient::new(
            rc_config,
            host.try_into().map_err(|e: InvalidDnsNameError| {
                Error::connection("invalid host address", e.to_string().some_box())
            })?,
        )
        .map_err(|e| Error::connection("could not connect to server", e.some_box()))
    }

    fn inner(&mut self) -> &mut Inner;

    fn read_buf<T>(&mut self, size: T) -> Result<Vec<u8>>
    where
        T: Into<usize>,
    {
        let mut buffer = vec![0; size.into()];
        self.inner()
            .read(&mut buffer)
            .map_err(|_e| Error::server("expected server response"))?;

        Ok(buffer)
    }

    fn try_read_buf<T>(&mut self, size: T) -> Result<Vec<u8>>
    where
        T: TryInto<usize>,
    {
        let mut buffer = vec![
            0;
            size.try_into().map_err(|_e| Error::client(
                "could not convert buffer length to usize"
            ))?
        ];
        self.inner()
            .read(&mut buffer)
            .map_err(|_e| Error::server("expected server response"))?;

        Ok(buffer)
    }

    fn empty_buffer() -> Vec<u8>;

    fn send_request(&mut self, request: RequestBuilder) -> Result<Response>;
}
