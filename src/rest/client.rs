pub mod auth;
mod config;
#[cfg(feature = "interpreter")]
mod interpreter;

#[cfg(feature = "interpreter")]
use interpreter::Interpreter;
use std::thread;
use std::time::Duration;

#[cfg(feature = "http2")]
pub use crate::http::Protocol::HTTP2;
pub use crate::http::Protocol::{self, HTTP1};
use crate::rest::client::config::Config;
pub use crate::rest::client::config::ConfigBuilder;
use crate::rest::{
    request::{InnerRequest, Request},
    response::Response,
    Error, Result,
};
use crate::HttpClient;
pub use auth::Credentials;

pub type Auth = auth::Authentication;
pub type AuthBuilder = auth::Builder;
pub type AuthMethod = auth::Method;
pub type AuthPlacement = auth::Placement;

pub type Success = ();

pub struct Client {
    pub(crate) inner: HttpClient,
    pub config: Config,
    pub access: Option<auth::Access>,
    #[cfg(feature = "interpreter")]
    pub interpreter: Interpreter,
}

impl From<ConfigBuilder> for Client {
    fn from(config: ConfigBuilder) -> Self {
        Self {
            inner: HttpClient::new(),
            config: config.into(),
            access: None,
            #[cfg(feature = "interpreter")]
            interpreter: Interpreter::default(),
        }
    }
}

impl Client {
    pub fn new(base_url: &str) -> Self {
        ConfigBuilder::from(base_url).into()
    }
    pub fn config() -> ConfigBuilder {
        ConfigBuilder::default()
    }
    pub fn auth() -> AuthBuilder {
        AuthBuilder::new()
    }

    pub fn execute(&mut self, request: InnerRequest) -> Result<Response> {
        let response = self.inner.execute(request).map_err(|e| Error::from(e))?;

        Ok(Response::from(response))
    }

    pub(crate) fn will_retry(&self) -> bool {
        !self.config.backoff_proc.retry_codes.is_empty()
    }

    pub fn try_execute(&mut self, request: InnerRequest) -> Result<Response> {
        let mut response = self.execute(request.clone())?;
        let mut retry_count = 0;
        while response.is_error() && self.config.backoff_proc.max_retries > retry_count {
            let status = response.status();
            if self.config.backoff_proc.retry_codes.contains(status) {
                if *status == 403 {
                    Request::refresh_access(self)?;
                }
                retry_count += 1;
                response = self.retry(request.clone(), retry_count)?;
            } else {
                return response.into();
            }
        }

        Ok(response)
    }

    fn retry(&mut self, request: InnerRequest, attempt_no: u8) -> Result<Response> {
        thread::sleep(Duration::from_millis(
            self.config.backoff_proc.calc(attempt_no),
        ));

        self.execute(request)
    }

    pub fn get(&mut self, url: &str) -> Request {
        let url = self.with_resource(url);
        Request::new(
            InnerRequest::get(&url).protocol(self.config.default_protocol),
            self,
        )
    }

    pub fn post(&mut self, url: &str) -> Request {
        let url = self.with_resource(url);
        Request::new(
            InnerRequest::post(&url).protocol(self.config.default_protocol),
            self,
        )
    }

    pub fn put(&mut self, url: &str) -> Request {
        let url = self.with_resource(url);
        Request::new(
            InnerRequest::put(&url).protocol(self.config.default_protocol),
            self,
        )
    }

    pub fn patch(&mut self, url: &str) -> Request {
        let url = self.with_resource(url);
        Request::new(
            InnerRequest::patch(&url).protocol(self.config.default_protocol),
            self,
        )
    }

    pub fn delete(&mut self, url: &str) -> Request {
        let url = self.with_resource(url);
        Request::new(
            InnerRequest::delete(&url).protocol(self.config.default_protocol),
            self,
        )
    }

    pub fn connect(&mut self, url: &str) -> Request {
        let url = self.with_resource(url);
        Request::new(
            InnerRequest::connect(&url).protocol(self.config.default_protocol),
            self,
        )
    }

    fn with_resource(&mut self, url: &str) -> String {
        match url.starts_with("/") {
            true => format!("{}{url}", self.config.base_url),
            false => format!("{}/{url}", self.config.base_url),
        }
    }

    #[cfg(feature = "http2")]
    pub fn set_http1(&mut self) -> Result<Success> {
        self.inner
            .connect_proto(&self.config.base_url, Protocol::HTTP1)
            .map_err(|e| Error::from(e))
    }

    #[cfg(feature = "interpreter")]
    pub fn all_optional_fields(&mut self) {
        self.interpreter
            .change_opt_field_proc(interpreter::OptionalFieldProc::AllOptional);
    }

    #[cfg(feature = "interpreter")]
    pub fn no_optional_fields(&mut self) {
        self.interpreter
            .change_opt_field_proc(interpreter::OptionalFieldProc::AllDefault);
    }

    #[cfg(feature = "interpreter")]
    pub fn individual_files(&mut self) {
        self.interpreter
            .change_mod_structure(interpreter::ModStructure::IndividualSrcFiles);
    }

    #[cfg(feature = "interpreter")]
    pub fn model_folder(&mut self, folder_name: &str) {
        self.interpreter
            .change_mod_structure(interpreter::ModStructure::OneModFolder(
                folder_name.to_string(),
            ));
    }
    #[cfg(feature = "interpreter")]
    pub fn interpret(
        &mut self,
        response: serde_json::Value,
        as_named_object: Option<&str>,
    ) -> Result<()> {
        self.interpreter.read_response(response, as_named_object)
    }
}
