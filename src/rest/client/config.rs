use crate::http::url::Url;
use crate::http::Protocol;
use crate::rest::client::auth::Grant;
#[cfg(feature = "http2")]
use crate::rest::client::HTTP1;
use crate::rest::client::{Auth, AuthBuilder};
use crate::rest::{Client, Error, Result};
use std::collections::HashMap;

pub struct Config {
    pub auth: Option<Auth>,
    pub backoff_proc: BackOffProcedure,
    pub base_url: Url,
    pub required_headers: HashMap<String, String>,
    pub default_protocol: Protocol,
}

impl From<ConfigBuilder> for Config {
    fn from(builder: ConfigBuilder) -> Self {
        Self {
            auth: match builder.auth {
                Some(auth) => Some(auth.finalize()),
                None => None,
            },
            backoff_proc: builder.backoff_proc,
            base_url: builder.base_url,
            required_headers: builder.required_headers,
            default_protocol: builder.default_protocol,
        }
    }
}

pub struct ConfigBuilder {
    pub(crate) auth: Option<AuthBuilder>,
    pub(crate) backoff_proc: BackOffProcedure,
    pub(crate) base_url: Url,
    pub(crate) required_headers: HashMap<String, String>,
    pub(crate) default_protocol: Protocol,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            backoff_proc: BackOffProcedure::default(),
            base_url: "".into(),
            required_headers: Default::default(),
            auth: None,
            default_protocol: Default::default(),
        }
    }
}

impl From<&str> for ConfigBuilder {
    fn from(base_url: &str) -> Self {
        Self {
            backoff_proc: Default::default(),
            base_url: base_url.into(),
            required_headers: Default::default(),
            auth: None,
            default_protocol: Default::default(),
        }
    }
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn build(self) -> Client {
        self.into()
    }
    pub fn preconnect(self) -> Result<Client> {
        let mut client = self.build();
        client
            .inner
            .connect_proto(&client.config.base_url, client.config.default_protocol)
            .map_err(|e| Error::from(e))?;

        Ok(client)
    }
    #[cfg(feature = "http2")]
    pub fn http1_only(self) -> Self {
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: self.auth,
            default_protocol: HTTP1,
        }
    }
    pub fn backoff(self, proc: BackOffProcedure) -> Self {
        Self {
            backoff_proc: proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: self.auth,
            default_protocol: self.default_protocol,
        }
    }
    pub fn base_url(self, url: &str) -> Self {
        Self {
            backoff_proc: self.backoff_proc,
            base_url: url.into(),
            required_headers: self.required_headers,
            auth: self.auth,
            default_protocol: self.default_protocol,
        }
    }
    pub fn required_headers(self, headers: &[(&str, &str)]) -> Self {
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            auth: self.auth,
            default_protocol: self.default_protocol,
        }
    }
    pub fn required_header(mut self, header: (&str, &str)) -> Self {
        let (key, value) = header;
        self.required_headers
            .insert(key.to_string(), value.to_string());
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: self.auth,
            default_protocol: self.default_protocol,
        }
    }
    pub fn auth(self, auth: AuthBuilder) -> Self {
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: Some(auth),
            default_protocol: self.default_protocol,
        }
    }

    pub fn basic_auth(self) -> Self {
        let auth = self.set_auth().basic();
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: Some(auth),
            default_protocol: self.default_protocol,
        }
    }

    pub fn bearer_auth(self) -> Self {
        let auth = self.set_auth().bearer();
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: Some(auth),
            default_protocol: self.default_protocol,
        }
    }

    pub fn oauth1(self, key: &str, token: &str) -> Self {
        let auth = self.set_auth().oauth1(key, token);
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: Some(auth),
            default_protocol: self.default_protocol,
        }
    }

    pub fn refresh_token(self, token: &str) -> Self {
        let auth = self.set_auth().refresh_token(token);
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: Some(auth),
            default_protocol: self.default_protocol,
        }
    }

    pub fn user_login(self, username: &str, password: &str) -> Self {
        let auth = self.set_auth().user_login(username, password);
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: Some(auth),
            default_protocol: self.default_protocol,
        }
    }

    pub fn email_login(self, email: &str, password: &str) -> Self {
        let auth = self.set_auth().email_login(email, password);
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: Some(auth),
            default_protocol: self.default_protocol,
        }
    }

    pub fn client_credentials(self, client_id: &str, client_secret: &str) -> Self {
        let auth = self.set_auth().client_credentials(client_id, client_secret);
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: Some(auth),
            default_protocol: self.default_protocol,
        }
    }

    pub fn key_token(self, key: &str, token: &str) -> Self {
        let auth = self.set_auth().key_token(key, token);
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: Some(auth),
            default_protocol: self.default_protocol,
        }
    }

    pub fn custom_credentials(self, credentials: Vec<(&str, &str)>) -> Self {
        let auth = self.set_auth().custom_credentials(credentials);
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: Some(auth),
            default_protocol: self.default_protocol,
        }
    }

    pub fn auth_url(self, url: &str) -> Self {
        let auth = self.set_auth().url(url);
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: Some(auth),
            default_protocol: self.default_protocol,
        }
    }

    pub fn auth_grant(self, grant: Grant) -> Self {
        let auth = self.set_auth().with_grant(grant);
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: Some(auth),
            default_protocol: self.default_protocol,
        }
    }

    fn set_auth(&self) -> AuthBuilder {
        match &self.auth {
            Some(auth) => auth.clone(),
            None => Auth::new(),
        }
    }
}

pub struct BackOffProcedure {
    pub(crate) retry_codes: Vec<u16>,
    pub(crate) operation: Box<dyn Fn(u8) -> u64>,
    pub(crate) cache: HashMap<u8, u64>,
    pub(crate) max_retries: u8,
}

impl Default for BackOffProcedure {
    fn default() -> Self {
        Self::new(Box::new(std_sleep), vec![429, 403], 3)
    }
}

impl BackOffProcedure {
    pub fn new(operation: Box<dyn Fn(u8) -> u64>, retry_codes: Vec<u16>, max_retries: u8) -> Self {
        let cache: HashMap<u8, u64> = (0..max_retries)
            .into_iter()
            .map(|a| (a, (operation)(a)))
            .collect();
        Self {
            operation,
            cache,
            max_retries,
            retry_codes,
        }
    }
    pub fn prefill(&mut self, attempts: u8) {
        (0..attempts).into_iter().for_each(|a| {
            self.calc(a);
        });
    }
    pub fn calc(&mut self, input: u8) -> u64 {
        match self.cache.get(&input) {
            Some(value) => *value,
            None => {
                let cached_input = input;
                let result: u64 = (self.operation)(input);
                let cached_result = result;
                self.cache.insert(cached_input, cached_result);
                result
            }
        }
    }
    pub fn clear_cache(&mut self) {
        self.cache.clear()
    }
    pub fn retry_codes(&mut self, codes: Vec<u16>) {
        self.retry_codes = codes
    }
    pub fn no_retry(&mut self) {
        self.retry_codes.clear();
        self.retry_codes.shrink_to(0);
    }
}

fn std_sleep(attempt_no: u8) -> u64 {
    2 ^ attempt_no as u64 * 100
}
