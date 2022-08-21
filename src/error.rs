use std::fmt::{self, Display};

pub struct Error {
    pub message: String,
}

impl Error {
    pub(crate) fn err<O, M>(message: M) -> Result<O, Error>
    where
        M: Into<Error>,
    {
        Err(message.into())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(&self.message)
    }
}

impl From<Error> for String {
    fn from(error: Error) -> String {
        error.message
    }
}

impl From<String> for Error {
    fn from(message: String) -> Error {
        Error { message }
    }
}

impl From<&str> for Error {
    fn from(message: &str) -> Error {
        Error {
            message: message.to_owned(),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        error.to_string().into()
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        error.to_string().into()
    }
}
