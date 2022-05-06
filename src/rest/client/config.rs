use crate::rest::client::auth::Authentication;
use crate::rest::client::Auth;
use std::collections::HashMap;

pub struct Config {
    pub(crate) backoff_proc: BackOffProcedure,
    pub(crate) base_url: String,
    pub(crate) required_headers: HashMap<String, String>,
    pub(crate) auth: Option<Authentication>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backoff_proc: BackOffProcedure::default(),
            base_url: "".to_string(),
            required_headers: Default::default(),
            auth: None,
        }
    }
}

impl From<&str> for Config {
    fn from(base_url: &str) -> Self {
        Self {
            backoff_proc: Default::default(),
            base_url: base_url.to_string(),
            required_headers: Default::default(),
            auth: None,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn backoff(self, proc: BackOffProcedure) -> Self {
        Self {
            backoff_proc: proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: self.auth,
        }
    }
    pub fn base_url(self, url: &str) -> Self {
        Self {
            backoff_proc: self.backoff_proc,
            base_url: url.to_string(),
            required_headers: self.required_headers,
            auth: self.auth,
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
        }
    }
    pub fn auth(self, auth: Auth) -> Self {
        Self {
            backoff_proc: self.backoff_proc,
            base_url: self.base_url,
            required_headers: self.required_headers,
            auth: Some(auth),
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
