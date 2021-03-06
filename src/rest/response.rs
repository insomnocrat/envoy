pub use crate::http::request::RequestBuilder;
pub use crate::http::Response as InnerResponse;
use crate::rest::error::SomeError;
use crate::{
    http::utf8_utils::UTF8Utils,
    rest::{Error, ErrorKind, HttpError, Result},
};
use serde::de::DeserializeOwned;
use serde_json;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    inner: InnerResponse,
}

impl From<InnerResponse> for Response {
    fn from(inner: InnerResponse) -> Self {
        Self { inner }
    }
}

impl TryFrom<std::result::Result<InnerResponse, HttpError>> for Response {
    type Error = Error;

    fn try_from(value: std::result::Result<InnerResponse, HttpError>) -> Result<Self> {
        Self { inner: value? }.into()
    }
}

impl From<Response> for Result<Response> {
    fn from(r: Response) -> Self {
        match r.is_ok() {
            true => Ok(r),
            false => Err(Error::new(&r.text(), ErrorKind::Status(*r.status()), None)),
        }
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl Response {
    pub fn json<T>(&self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        serde_json::from_slice(&self.inner.body)
            .map_err(|e| Error::new("could not parse json", ErrorKind::Parse, e.some_box()))
    }
    pub fn is_error(&self) -> bool {
        self.inner.status_code != 200
    }
    pub fn is_ok(&self) -> bool {
        self.inner.status_code >= 200 && self.inner.status_code < 300
    }
    pub fn text(&self) -> String {
        self.inner.body.as_utf8_lossy().to_string()
    }
    pub fn utf8(&self) -> Result<String> {
        self.inner.body.as_utf8().map_err(|e| {
            let http_error: HttpError = e.into();
            http_error.into()
        })
    }
    pub fn status(&self) -> &u16 {
        &self.inner.status_code
    }
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.inner.headers
    }
    pub fn assert(&self) {
        assert!(self.is_ok())
    }
    pub fn assert_err(&self) {
        assert!(self.is_error())
    }
}
