#[cfg(multihost)]
use super::pool::HostPool;
use super::{connection::Connection, request::Http1Request, Response, Result};

pub struct Client {
    connection: Option<Connection>,
}
impl Client {
    pub fn new() -> Self {
        Self { connection: None }
    }

    pub fn execute(&mut self, request: Http1Request) -> Result<Response> {
        let host = request.host();
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

    pub fn execute(&mut self, request: Http1Request) -> Result<Response> {
        self.pool.send_request(request)?;
        self.pool.fetch_response()
    }
}
