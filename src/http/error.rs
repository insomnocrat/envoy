use std::any::Any;
use std::fmt::{Display, Formatter};
use std::string::FromUtf8Error;

#[derive(Debug)]
pub struct Error {
    pub(crate) kind: ErrorKind,
    pub(crate) message: String,
}

impl Error {
    pub fn new(message: &str, kind: ErrorKind) -> Self {
        Self {
            message: message.to_string(),
            kind,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: {}\nKind: {:?}", self.message, self.kind)
    }
}

impl std::error::Error for Error {}

#[derive(Debug)]
pub enum ErrorKind {
    Thread(Option<Box<dyn Any + Send>>),
    User,
    Client,
    Server,
    Connection(Option<Box<dyn Any + Send>>),
}

pub trait SomeError {
    fn some_box(self) -> Option<Box<dyn Any + Send>>;
}

impl<T: 'static + Sized + Send> SomeError for T {
    fn some_box(self) -> Option<Box<dyn Any + Send>> {
        Some(Box::new(self))
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::new(
            "could not connect to server",
            ErrorKind::Connection(e.some_box()),
        )
    }
}

impl From<FromUtf8Error> for Error {
    fn from(_: FromUtf8Error) -> Self {
        Self::new("response contained invalid utf-8", ErrorKind::Server)
    }
}

