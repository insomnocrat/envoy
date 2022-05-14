use super::{pooled_conn::PooledConn, Response, Result};
use crate::http::request::RequestBuilder;

pub struct Client {
    connection: Option<PooledConn>,
}
impl Client {
    pub fn new() -> Self {
        Self { connection: None }
    }

    pub fn execute(&mut self, request: RequestBuilder) -> Result<Response> {
        let host = request.url.authority();
        let connection = match &mut self.connection {
            Some(conn) => conn,
            None => self.connection.insert(PooledConn::new(&host)?),
        };
        if !connection.host.eq(&host) {
            connection.join_thread();
            *connection = PooledConn::new(&host)?;
        }
        connection.send_request(request)?;

        connection.check_response()
    }

    pub fn connect(&mut self, host: &str) -> Result<()> {
        self.connection = Some(PooledConn::new(&host)?);

        Ok(())
    }
}
