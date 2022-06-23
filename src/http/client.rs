use super::{pooled_conn::PooledConn, Response, Result};
use crate::http::request::RequestBuilder;
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

    pub fn connect(&mut self, host: &str) -> Result<()> {
        let conn = match PooledConn::new(&host, Protocol::default()) {
            Ok(c) => c,
            Err(e) => {
                if let ErrorKind::Protocol = e.kind {
                    PooledConn::new(&host, Protocol::HTTP1)?
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
}
