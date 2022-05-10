#[cfg(multihost)]
use super::pool::HostPool;
use super::{connection::Connection, Response, Result};
use crate::http::proto_conn::ProtoConn;
use crate::http::request::RequestBuilder;

pub struct Client<T: ProtoConn> {
    connection: Option<Connection<T>>,
}
impl<T: 'static + ProtoConn> Client<T> {
    pub fn new() -> Self {
        Self { connection: None }
    }

    pub fn execute(&mut self, request: RequestBuilder) -> Result<Response> {
        let host = request.url.authority();
        let connection = match &mut self.connection {
            Some(conn) => conn,
            None => self.connection.insert(Connection::new(&host)?),
        };
        if !connection.host.eq(&host) {
            connection.join_thread();
            *connection = Connection::new(&host)?;
        }
        connection.send_request(request)?;

        connection.check_response()
    }

    pub fn connect(&mut self, host: &str) -> Result<()> {
        self.connection = Some(Connection::new(&host)?);

        Ok(())
    }
}

#[cfg(feature = "multihost")]
pub struct ClientMultiHost {
    pool: HostPool,
}
#[cfg(feature = "multihost")]
impl ClientMultiHost {
    pub fn new() -> Self {
        Self {
            pool: HostPool::new(),
        }
    }

    pub fn execute(&mut self, request: RequestBuilder) -> Result<Response> {
        self.pool.send_request(request)?;
        self.pool.fetch_response()
    }
}
