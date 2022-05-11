pub mod http;
#[cfg(feature = "rest")]
pub mod rest;

pub type HttpClient = http::HttpClient;
#[cfg(feature = "rest")]
pub type RestClient = rest::Client;
