pub(crate) mod auth;
mod config;
mod handlers;
#[cfg(feature = "interpreter")]
mod interpreter;
#[cfg(feature = "interpreter")]
use interpreter::Interpreter;

use crate::http::Http1Stream;
pub use crate::rest::client::config::Config;
use crate::rest::{
    request::{InnerRequest, Request},
    response::Response,
    Error, Result,
};
use crate::HttpClient;
use std::{thread, time};

pub type Auth = auth::Authentication;
pub type AuthMethod = auth::Method;
pub type AuthPlacement = auth::Placement;

pub type Success = ();

pub struct Client {
    pub(crate) inner: HttpClient<Http1Stream>,
    pub(crate) config: Config,
    pub(crate) access: Option<auth::Access>,
    #[cfg(feature = "interpreter")]
    pub interpreter: Interpreter,
}

impl From<Config> for Client {
    fn from(config: Config) -> Self {
        Self {
            inner: HttpClient::new(),
            config,
            access: None,
            #[cfg(feature = "interpreter")]
            interpreter: Interpreter::default(),
        }
    }
}

impl Client {
    pub fn new(base_url: &str) -> Self {
        Config::from(base_url).into()
    }
    pub fn config() -> Config {
        Config::default()
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
        thread::sleep(time::Duration::from_millis(
            self.config.backoff_proc.calc(attempt_no),
        ));

        self.execute(request)
    }

    pub fn get(&mut self, url: &str) -> Request {
        let url = self.with_resource(url);
        Request::new(InnerRequest::get(&url), self)
    }

    pub fn post(&mut self, url: &str) -> Request {
        let url = self.with_resource(url);
        Request::new(InnerRequest::post(&url), self)
    }

    pub fn put(&mut self, url: &str) -> Request {
        let url = self.with_resource(url);
        Request::new(InnerRequest::put(&url), self)
    }

    pub fn patch(&mut self, url: &str) -> Request {
        let url = self.with_resource(url);
        Request::new(InnerRequest::patch(&url), self)
    }

    pub fn delete(&mut self, url: &str) -> Request {
        let url = self.with_resource(url);
        Request::new(InnerRequest::delete(&url), self)
    }

    fn with_resource(&mut self, url: &str) -> String {
        match url.starts_with("/") {
            true => format!("{}{url}", self.config.base_url),
            false => format!("{}/{url}", self.config.base_url),
        }
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
