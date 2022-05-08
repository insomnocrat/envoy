#[cfg(multihost)]
use super::pool::HostPool;
use super::{connection::Connection, Response, Result};
use crate::http::http1::stream::Http1Stream;
use crate::http::request::RequestBuilder;

pub struct Client {
    connection: Option<Connection<Http1Stream>>,
}
impl Client {
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
