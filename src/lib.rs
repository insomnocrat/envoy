pub mod http;
#[cfg(feature = "rest")]
pub mod rest;

pub type HttpClient = http::client::Client;
#[cfg(feature = "rest")]
pub type RestClient = rest::Client;
