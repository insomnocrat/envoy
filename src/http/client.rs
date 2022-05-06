#[cfg(multihost)]
use super::pool::HostPool;
use super::{connection::ManagedConnection, request::Request, Response, Result};

pub struct Client {
    connection: Option<ManagedConnection>,
}
impl Client {
    pub fn new() -> Self {
        Self { connection: None }
    }

    pub fn execute(&mut self, request: Request) -> Result<Response> {
        let host = request.host();
        let hostname = request.hostname();
        let connection = self
            .connection
            .get_or_insert(ManagedConnection::new(&host, hostname)?);
        if !connection.host.eq(&host) {
            connection.join_thread();
            *connection = ManagedConnection::new(&host, hostname)?;
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

    pub fn execute(&mut self, request: Request) -> Result<Response> {
        self.pool.send_request(request)?;
        self.pool.fetch_response()
    }
}
