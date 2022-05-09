pub mod http;
#[cfg(feature = "rest")]
pub mod rest;

pub type HttpClient<T> = http::client::Client<T>;
#[cfg(feature = "rest")]
pub type RestClient = rest::Client;
