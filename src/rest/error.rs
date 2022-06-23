use crate::rest::{HttpError, HttpErrorKind};
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct Error {
    kind: Kind,
    message: String,
    source: Option<Box<dyn std::error::Error>>,
}

impl Error {
    pub fn new(message: &str, kind: Kind, source: Option<Box<dyn std::error::Error>>) -> Self {
        Self {
            message: message.to_string(),
            kind,
            source,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: {}\nKind: {:?}", self.message, self.kind)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref()
    }
}

impl From<HttpError> for Error {
    fn from(e: HttpError) -> Self {
        let message = e.message.to_string();
        let kind = match &e.kind {
            HttpErrorKind::Client
            | HttpErrorKind::User
            | HttpErrorKind::Thread(_)
            | HttpErrorKind::Protocol => Kind::Protocol,
            HttpErrorKind::Server => Kind::Server,
            HttpErrorKind::Connection(_) => Kind::Connection,
        };

        Self::new(&message, kind, e.some_box())
    }
}

#[derive(Debug)]
pub enum Kind {
    Parse,
    Status(u16),
    Connection,
    Server,
    Client,
    Protocol,
    #[cfg(feature = "interpreter")]
    Interpreter,
}

pub trait SomeError {
    fn some_box(self) -> Option<Box<dyn std::error::Error>>;
}

impl<T: 'static> SomeError for T
where
    T: std::error::Error + Sized,
{
    fn some_box(self) -> Option<Box<dyn std::error::Error>> {
        Some(Box::new(self))
    }
}
