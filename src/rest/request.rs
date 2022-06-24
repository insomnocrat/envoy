use crate::http::request::headers::AUTHORIZATION;
pub use crate::http::request::headers::*;
pub use crate::http::request::RequestBuilder as InnerRequest;
#[cfg(feature = "multipart")]
use crate::http::utf8_utils::UTF8Utils;
pub use crate::http::Response as InnerResponse;
use crate::rest::client::auth::OAUTH;
use crate::rest::client::{
    auth::AccessTokenResponse, auth::BASIC, auth::BEARER, AuthMethod, AuthPlacement, Success,
};
use crate::rest::error::SomeError;
#[cfg(feature = "multipart")]
use crate::rest::multipart::MultipartForm;
use crate::rest::request::values::{ALL, JSON};
use crate::{
    rest::{response::Response, Error, ErrorKind, Result},
    RestClient,
};
use base64::encode;
use serde::{de::DeserializeOwned, Serialize};
use serde_json;

pub struct Request<'a> {
    pub(crate) inner: InnerRequest,
    client_ref: &'a mut RestClient,
}

impl<'a> Request<'a> {
    pub fn send(mut self) -> Result<Response> {
        self.set_auth()?;
        self.set_required_headers();
        match self.client_ref.will_retry() {
            true => self.client_ref.try_execute(self.inner),
            false => self.client_ref.execute(self.inner),
        }
    }

    pub fn expect_json<T>(self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.accept_json().send()?.json()
    }

    pub fn expect_utf8(self) -> Result<String> {
        self.send()?.utf8()
    }

    #[cfg(feature = "interpreter")]
    pub fn expect_model(self) -> Result<serde_json::Value> {
        self.expect_json()
    }

    pub(crate) fn new(inner: InnerRequest, client_ref: &'a mut RestClient) -> Self {
        Self { inner, client_ref }
    }

    pub fn body<T: Serialize>(self, body: &T) -> Self {
        let body = serde_json::to_vec(body).unwrap();
        Self {
            inner: self.inner.body(&body).header((CONTENT_TYPE, JSON)),
            client_ref: self.client_ref,
        }
    }

    pub fn query<T: serde::Serialize + Sized>(mut self, query: T) -> Self {
        let query = serde_urlencoded::to_string(query).unwrap();
        self.inner.set_query(query.into_bytes());
        Self {
            inner: self.inner,
            client_ref: self.client_ref,
        }
    }

    pub fn opt_query<T: serde::Serialize + Sized>(self, query: Option<T>) -> Self {
        match query {
            Some(query) => self.query(query),
            None => self,
        }
    }

    #[cfg(feature = "multipart")]
    pub fn multipart_body<T: AsRef<[u8]>>(mut self, body: &[(T, T)]) -> Self {
        let body = MultipartForm::from(body);
        self.extend_header(
            CONTENT_TYPE,
            format!(
                "multipart/form-data; boundary={}",
                body.boundary.as_utf8_lossy()
            )
            .as_bytes(),
        );

        Self {
            inner: self.inner.body(body.to_bytes().as_slice()),
            client_ref: self.client_ref,
        }
    }

    pub fn header(self, key: &[u8], value: &[u8]) -> Self {
        Self {
            inner: self.inner.header((key, value)),
            client_ref: self.client_ref,
        }
    }

    pub fn headers(self, headers: Vec<(&[u8], &[u8])>) -> Self {
        Self {
            inner: self.inner.headers(headers),
            client_ref: self.client_ref,
        }
    }

    pub fn extend_header(&mut self, key: &[u8], value: &[u8]) {
        self.inner.insert_header((key, &value));
    }
    pub fn extend_headers(&mut self, headers: Vec<(&[u8], &[u8])>) {
        self.inner.extend_headers(headers);
    }
    pub fn body_mut<T: Serialize>(&mut self, body: &T) {
        let body = serde_json::to_vec(body).unwrap();
        self.inner.body_mut(&body);
    }

    fn set_required_headers(&mut self) {
        self.inner.extend_headers(
            self.client_ref
                .config
                .required_headers
                .iter()
                .map(|(k, v)| (k.as_bytes(), v.as_bytes()))
                .collect(),
        );
    }

    fn encode_basic_auth(username: &[u8], password: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let mut value = username.to_vec();
        value.push(0x3A);
        value.extend_from_slice(password);
        let mut basic = BASIC.to_vec();
        basic.extend_from_slice(encode(value).as_bytes());

        (AUTHORIZATION.to_vec(), basic)
    }

    fn set_oauth1(request: &mut InnerRequest, key: &str, token: &str) {
        let mut value = OAUTH.to_vec();
        value.extend(
            format!("oauth_consumer_key=\"{}\", oauth_token=\"{}\"", key, token).as_bytes(),
        );
        request.insert_header((AUTHORIZATION, &value));
    }

    fn set_basic_auth(request: &mut InnerRequest, username: &str, password: Option<&str>) {
        let (header, value) = Self::encode_basic_auth(
            username.as_bytes(),
            password.map(|p| p.as_bytes()).unwrap_or_default(),
        );
        request.insert_header((&header, &value))
    }

    fn set_bearer_auth(request: &mut InnerRequest, token: &str) {
        let mut value = BEARER.to_vec();
        value.extend(encode(token).as_bytes());
        request.insert_header((AUTHORIZATION, &value));
    }

    fn set_auth(&mut self) -> Result<Success> {
        if let Some(auth) = &self.client_ref.config.auth {
            match &auth.method {
                AuthMethod::BEARER => {
                    let mut access = match &self.client_ref.access {
                        Some(access) => access,
                        None => return Err(Error::new("no bearer token", ErrorKind::Client, None)),
                    };
                    if access.is_expired() {
                        Self::refresh_access(&mut self.client_ref)?;
                        match &self.client_ref.access {
                            Some(a) => access = a,
                            None => {
                                return Err(Error::new("no bearer token", ErrorKind::Client, None))
                            }
                        }
                    }
                    Self::set_bearer_auth(&mut self.inner, &access.token);
                }
                AuthMethod::BASIC => {
                    let (username, password) = auth.credentials.to_basic()?;
                    Self::set_basic_auth(&mut self.inner, username, password);
                }
                AuthMethod::OAUTH => {
                    let key = auth
                        .credentials
                        .value_map
                        .get("oauth_consumer_key")
                        .ok_or_else(|| {
                            Error::new("invalid oauth credentials", ErrorKind::Client, None)
                        })?;
                    let token = auth
                        .credentials
                        .value_map
                        .get("oauth_token")
                        .ok_or_else(|| {
                            Error::new("invalid oauth credentials", ErrorKind::Client, None)
                        })?;
                    Self::set_oauth1(&mut self.inner, key, token);
                }
                AuthMethod::OTHER => match &auth.credentials.placement {
                    AuthPlacement::HEADER => {
                        self.inner.extend_headers(
                            auth.credentials
                                .value_map
                                .iter()
                                .map(|(k, v)| (k.as_bytes(), v.as_bytes()))
                                .collect(),
                        );
                    }
                    AuthPlacement::BODY => self.inner.body_mut(
                        serde_json::to_vec(&auth.credentials.value_map)
                            .map_err(|e| {
                                Error::new(
                                    "could not parse credentials",
                                    ErrorKind::Client,
                                    e.some_box(),
                                )
                            })?
                            .as_slice(),
                    ),
                    AuthPlacement::QUERY => {
                        let mut query_pairs = Vec::with_capacity(auth.credentials.value_map.len());
                        for (key, value) in auth.credentials.value_map.iter() {
                            query_pairs.push((key.as_bytes(), value.as_bytes()));
                        }
                        self.inner.extend_query(query_pairs);
                    }
                },
            }
        }

        Ok(())
    }

    pub(crate) fn refresh_access(client_ref: &mut RestClient) -> Result<Success> {
        let auth = match &client_ref.config.auth {
            Some(auth) => auth,
            None => return Ok(()),
        };
        let mut auth_request = match &auth.method {
            AuthMethod::BEARER => InnerRequest::post(&auth.url),
            _ => return Ok(()),
        };
        let credentials = match &auth.grant {
            Some(grant) => {
                if grant.use_parent_auth_as_basic {
                    let (username, password) = auth.credentials.to_basic()?;
                    Self::set_basic_auth(&mut auth_request, username, password);
                }
                &grant.credentials
            }
            None => &auth.credentials,
        };
        match &credentials.placement {
            AuthPlacement::HEADER => {
                let (username, password) = credentials.to_basic()?;
                Self::set_basic_auth(&mut auth_request, username, password);
            }
            AuthPlacement::BODY => auth_request.body_mut(
                serde_json::to_vec(&credentials.value_map)
                    .map_err(|e| {
                        Error::new(
                            "could not parse credentials",
                            ErrorKind::Client,
                            e.some_box(),
                        )
                    })?
                    .as_slice(),
            ),
            AuthPlacement::QUERY => {
                let mut query_pairs = Vec::with_capacity(credentials.value_map.len());
                for (key, value) in credentials.value_map.iter() {
                    query_pairs.push((key.as_bytes(), value.as_bytes()));
                }
                auth_request.extend_query(query_pairs);
            }
        };
        let response: Result<Response> = Response::from(
            client_ref
                .inner
                .execute(auth_request)
                .map_err(|e| Error::from(e))?,
        )
        .into();
        let response = response?;
        client_ref.access = Some(response.json::<AccessTokenResponse>()?.into());

        Ok(())
    }

    pub fn accept_all(mut self) -> Self {
        self.inner.insert_header((ACCEPT, ALL));

        self
    }
    pub fn accept_json(mut self) -> Self {
        self.inner.insert_header((ACCEPT, JSON));

        self
    }

    pub fn basic_auth(mut self, username: &str, password: Option<&str>) -> Self {
        let (header, value) = Self::encode_basic_auth(
            username.as_bytes(),
            password.map(|p| p.as_bytes()).unwrap_or_default(),
        );
        self.inner.insert_header((&header, &value));

        self
    }

    pub fn bearer_auth(mut self, token: &str) -> Self {
        Self::set_bearer_auth(&mut self.inner, token);

        self
    }

    pub fn oauth1(mut self, key: &str, token: &str) -> Self {
        Self::set_oauth1(&mut self.inner, key, token);

        self
    }

    pub fn user_agent(mut self, agent: &str) -> Self {
        self.inner.insert_header((USER_AGENT, agent.as_bytes()));

        self
    }
}
