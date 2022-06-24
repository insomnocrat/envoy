mod bearer;
pub use bearer::Access;
pub use bearer::AccessTokenResponse;

use crate::rest::{Error, ErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(crate) const BASIC: &[u8; 6] = b"Basic ";
pub(crate) const BEARER: &[u8; 7] = b"Bearer ";
pub(crate) const OAUTH: &[u8] = b"OAuth ";
const CLIENT_ID: &str = "client_id";
const CLIENT_SECRET: &str = "client_secret";
const KEY: &str = "key";
const TOKEN: &str = "token";
const USERNAME: &str = "username";
const PASSWORD: &str = "password";
const EMAIL: &str = "email";
const OAUTH_CONSUMER_KEY: &str = "oauth_consumer_key";
const OAUTH_TOKEN: &str = "oauth_token";
const REFRESH_TOKEN: &str = "refresh_token";
const GRANT_TYPE: &str = "grant_type";
const SCOPE: &str = "scope";

#[derive(Clone, Debug)]
pub struct Authentication {
    pub method: Method,
    pub credentials: Credentials,
    pub url: String,
    pub grant: Option<Grant>,
}

impl Authentication {
    pub fn new() -> Builder {
        Builder::new()
    }
    pub fn basic() -> Builder {
        Builder::new().basic()
    }
    pub fn bearer() -> Builder {
        Builder::new().bearer()
    }
    pub fn oauth1(key: &str, token: &str) -> Builder {
        Builder::new().oauth1(key, token)
    }
}

#[derive(Clone, Debug)]
pub struct Builder {
    method: Method,
    credentials: Option<Credentials>,
    url: Option<String>,
    grant: Option<Grant>,
}

impl Builder {
    pub fn new() -> Self {
        Builder {
            method: Method::Other,
            credentials: None,
            url: None,
            grant: None,
        }
    }
    pub fn url(self, url: &str) -> Self {
        Builder {
            method: self.method,
            url: Some(url.to_string()),
            credentials: self.credentials,
            grant: self.grant,
        }
    }
    pub fn basic(self) -> Self {
        Builder {
            method: Method::Basic,
            url: self.url,
            credentials: self.credentials,
            grant: self.grant,
        }
    }
    pub fn bearer(self) -> Self {
        Builder {
            method: Method::Bearer,
            url: self.url,
            credentials: self.credentials,
            grant: self.grant,
        }
    }
    pub fn key_token(self, key: &str, token: &str) -> Self {
        Builder {
            method: self.method,
            credentials: Some(Credentials::key_token_pair(key, token)),
            url: self.url,
            grant: self.grant,
        }
    }
    pub fn custom_credentials(self, values: Vec<(&str, &str)>) -> Self {
        Builder {
            method: self.method,
            credentials: Some(Credentials::custom(values)),
            url: self.url,
            grant: self.grant,
        }
    }
    pub fn client_credentials(self, client_id: &str, client_secret: &str) -> Self {
        Builder {
            method: self.method,
            credentials: Some(Credentials::client_credentials(client_id, client_secret)),
            url: self.url,
            grant: self.grant,
        }
    }
    pub fn refresh_token(self, refresh_token: &str) -> Self {
        Builder {
            method: self.method,
            credentials: Some(Credentials::refresh_token(refresh_token)),
            url: self.url,
            grant: self.grant,
        }
    }
    pub fn user_login(self, username: &str, password: &str) -> Self {
        Builder {
            method: self.method,
            credentials: Some(Credentials::user_login(username, password)),
            url: self.url,
            grant: self.grant,
        }
    }
    pub fn email_login(self, email: &str, password: &str) -> Self {
        Builder {
            method: self.method,
            credentials: Some(Credentials::email_login(email, password)),
            url: self.url,
            grant: self.grant,
        }
    }
    pub fn oauth1(self, key: &str, token: &str) -> Self {
        Builder {
            method: Method::OAuth,
            credentials: Some(Credentials::oauth1(key, token)),
            url: self.url,
            grant: self.grant,
        }
    }
    pub fn with_grant(self, grant: Grant) -> Self {
        Builder {
            method: self.method,
            credentials: self.credentials,
            url: self.url,
            grant: Some(grant),
        }
    }
    pub fn credentials(self, credentials: Credentials) -> Self {
        Builder {
            method: self.method,
            credentials: Some(credentials),
            url: self.url,
            grant: self.grant,
        }
    }
    pub fn finalize(self) -> Authentication {
        let uri = match self.url {
            Some(i) => i,
            None => String::new(),
        };
        Authentication {
            method: self.method,
            credentials: self.credentials.unwrap(),
            url: uri,
            grant: self.grant,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub placement: Placement,
    pub kind: Kind,
    pub value_map: HashMap<String, String>,
}

impl Credentials {
    pub fn new(placement: Placement, kind: Kind, value_map: HashMap<String, String>) -> Self {
        Self {
            placement,
            kind,
            value_map,
        }
    }
    pub fn to_basic<'a>(&'a self) -> Result<(&'a str, Option<&'a str>)> {
        let deref = |p: &'a String| Some(&p[..]);
        let message = || Error::new("invalid basic auth", ErrorKind::Client, None);
        let result: (&str, Option<&str>) = match self.kind {
            Kind::UserLogin | Kind::Other => (
                self.value_map.get(USERNAME).ok_or_else(message)?,
                self.value_map.get(PASSWORD).and_then(deref),
            ),
            Kind::ClientCredentials => (
                self.value_map.get(CLIENT_ID).ok_or_else(message)?,
                self.value_map.get(CLIENT_SECRET).and_then(deref),
            ),
            Kind::EmailLogin => (
                self.value_map.get(EMAIL).ok_or_else(message)?,
                self.value_map.get(PASSWORD).and_then(deref),
            ),
            Kind::KeyTokenPair => (
                self.value_map.get(KEY).ok_or_else(message)?,
                self.value_map.get(TOKEN).and_then(deref),
            ),
            Kind::Oauth1 => (
                self.value_map.get(OAUTH_CONSUMER_KEY).ok_or_else(message)?,
                self.value_map.get(OAUTH_TOKEN).and_then(deref),
            ),
            Kind::RefreshToken => (self.value_map.get(TOKEN).ok_or_else(message)?, None),
        };

        Ok(result)
    }
    pub fn client_credentials(client_id: &str, client_secret: &str) -> Self {
        Self::new(
            Placement::default(),
            Kind::ClientCredentials,
            HashMap::from([
                (CLIENT_ID.to_string(), client_id.to_string()),
                (CLIENT_SECRET.to_string(), client_secret.to_string()),
            ]),
        )
    }
    pub fn user_login(username: &str, password: &str) -> Self {
        Self::new(
            Placement::default(),
            Kind::UserLogin,
            HashMap::from([
                (USERNAME.to_string(), username.to_string()),
                (PASSWORD.to_string(), password.to_string()),
            ]),
        )
    }
    pub fn email_login(email: &str, password: &str) -> Self {
        Self::new(
            Placement::default(),
            Kind::EmailLogin,
            HashMap::from([
                (EMAIL.to_string(), email.to_string()),
                (PASSWORD.to_string(), password.to_string()),
            ]),
        )
    }
    pub fn key_token_pair(key: &str, token: &str) -> Self {
        Self::new(
            Placement::default(),
            Kind::KeyTokenPair,
            HashMap::from([
                (KEY.to_string(), key.to_string()),
                (TOKEN.to_string(), token.to_string()),
            ]),
        )
    }
    pub fn refresh_token(token: &str) -> Self {
        Self::new(
            Placement::default(),
            Kind::RefreshToken,
            HashMap::from([(REFRESH_TOKEN.to_string(), token.to_string())]),
        )
    }
    pub fn oauth1(key: &str, token: &str) -> Self {
        Self::new(
            Placement::Header,
            Kind::Oauth1,
            HashMap::from([
                (OAUTH_CONSUMER_KEY.to_string(), key.to_string()),
                (OAUTH_TOKEN.to_string(), token.to_string()),
            ]),
        )
    }
    pub fn custom(values: Vec<(&str, &str)>) -> Self {
        let mut value_map = HashMap::new();
        for (key, value) in values {
            value_map.insert(key.to_string(), value.to_string());
        }
        Self::new(Placement::default(), Kind::Other, value_map)
    }
    pub fn with_value(self, key: &str, value: &str) -> Self {
        let mut value_map = self.value_map;
        value_map.insert(key.to_string(), value.to_string());
        Self {
            placement: self.placement,
            kind: self.kind,
            value_map,
        }
    }
    pub fn with_values(self, values: Vec<(&str, &str)>) -> Self {
        let mut value_map = self.value_map;
        for (key, value) in values {
            value_map.insert(key.to_string(), value.to_string());
        }
        Self {
            placement: self.placement,
            kind: self.kind,
            value_map,
        }
    }
    pub fn body(self) -> Self {
        Self::new(Placement::Body, self.kind, self.value_map)
    }
    pub fn header(self) -> Self {
        Self::new(Placement::Header, self.kind, self.value_map)
    }
    pub fn query(self) -> Self {
        Self::new(Placement::Query, self.kind, self.value_map)
    }
    pub fn body_urlencoded(self) -> Self {
        Self::new(Placement::UrlEncodedBody, self.kind, self.value_map)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Kind {
    ClientCredentials,
    RefreshToken,
    KeyTokenPair,
    UserLogin,
    EmailLogin,
    Oauth1,
    Other,
}

impl Kind {
    pub fn to_grant(&self) -> String {
        match &self {
            Self::ClientCredentials => "client_credentials".to_string(),
            Self::RefreshToken => "refresh_token".to_string(),
            Self::UserLogin | Self::EmailLogin => "password".to_string(),
            Self::KeyTokenPair | Self::Oauth1 => "token".to_string(),
            Self::Other => "unknown".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Placement {
    Body,
    Header,
    Query,
    UrlEncodedBody,
}
impl Default for Placement {
    fn default() -> Self {
        Self::Header
    }
}

#[derive(Clone, Debug)]
pub enum Method {
    Basic,
    Bearer,
    OAuth,
    Other,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Grant {
    pub scope: Option<String>,
    pub credentials: Credentials,
    pub use_parent_auth_as_basic: bool,
}

impl Grant {
    pub fn new(
        mut credentials: Credentials,
        scope: Option<String>,
        use_parent_auth_as_basic: bool,
    ) -> Self {
        let param = credentials.kind.to_grant();
        let param = match param.eq("unknown") {
            false => param,
            true => credentials
                .value_map
                .get("grant_type")
                .expect("custom grant type not specified")
                .to_string(),
        };

        if let Some(scope) = &scope {
            credentials = credentials.with_values(vec![(&GRANT_TYPE, &param), (&SCOPE, scope)]);
        } else {
            credentials = credentials.with_value(&GRANT_TYPE, &param);
        }
        Self {
            credentials,
            scope,
            use_parent_auth_as_basic,
        }
    }
}
