pub mod client;
mod error;
pub mod request;
pub mod response;
#[cfg(test)]
mod tests;

pub use error::Error;
pub use error::Kind as ErrorKind;
pub type Result<T> = std::result::Result<T, Error>;
pub type ClientConfig = client::Config;
pub use crate::http::error::ErrorKind as HttpErrorKind;
pub use crate::http::Error as HttpError;
pub use client::Client;
