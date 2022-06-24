use super::{pooled_conn::PooledConn, Response, Result};
use crate::http::request::RequestBuilder;
use crate::http::url::Url;
use crate::http::{Error, ErrorKind, Protocol};

pub struct Client {
    pooled_conn: Option<PooledConn>,
}
impl Client {
    pub fn new() -> Self {
        Self { pooled_conn: None }
    }

    pub fn execute(&mut self, request: RequestBuilder) -> Result<Response> {
        let host = request.url.authority();
        let connection = match &mut self.pooled_conn {
            Some(conn) => conn,
            None => self
                .pooled_conn
                .insert(PooledConn::new(&host, request.protocol)?),
        };
        if !connection.host.eq(&host) {
            connection.join_thread()?;
            *connection = PooledConn::new(&host, request.protocol)?;
        }
        connection.send_request(request)?;

        connection.check_response()
    }

    pub fn connect(&mut self, host: &Url) -> Result<()> {
        let conn = match PooledConn::new(&host.authority(), Protocol::default()) {
            Ok(c) => c,
            Err(e) => {
                if let ErrorKind::Protocol = e.kind {
                    PooledConn::new(&host.authority(), Protocol::HTTP1)?
                } else {
                    return Err(e);
                }
            }
        };
        self.pooled_conn = Some(conn);

        Ok(())
    }

    pub fn connect_proto(&mut self, host: &Url, protocol: Protocol) -> Result<()> {
        let conn = match PooledConn::new(&host.authority(), protocol) {
            Ok(c) => c,
            Err(e) => {
                if let ErrorKind::Protocol = e.kind {
                    PooledConn::new(&host.authority(), Protocol::HTTP1)?
                } else {
                    return Err(e);
                }
            }
        };
        self.pooled_conn = Some(conn);

        Ok(())
    }

    pub fn reset_connection(&mut self) -> Result<()> {
        if let Some(pooled) = &mut self.pooled_conn {
            if let Some(connection) = pooled.join_thread()? {
                pooled.spawn_connection(connection);
                return Ok(());
            }
        }

        Err(Error::user("attempted to reset non-existent connection"))
    }

    #[cfg(feature = "http2")]
    pub fn ping(&mut self) -> Result<std::time::Duration> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::client("time conversion error"))?
            .as_millis();
        match &mut self.pooled_conn {
            Some(connection) => {
                connection.ping_inner()?;
                let result = connection.check_response()?;
                let arrived = <u128>::from_be_bytes(
                    <[u8; 16]>::try_from(result.body.as_slice())
                        .map_err(|_| Error::client("time conversion error"))?,
                );
                let timing = std::time::Duration::from_millis((arrived - now) as u64);

                Ok(timing)
            }
            None => Err(Error::user("attempted to ping non-existent connection")),
        }
    }
}
