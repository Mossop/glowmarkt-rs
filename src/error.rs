//! A basic error container.

use std::fmt::{self, Display};

use reqwest::StatusCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The type of an error.
pub enum ErrorKind {
    /// The requested item was not found.
    NotFound,
    /// Authentication failed.
    NotAuthenticated,
    /// A network error.
    Network,
    /// An error likely caused by this crate.
    Client,
    /// An error on the server.
    Server,
    /// An error decoding the API response.
    Response,
}

/// A fairly generic error container.
pub struct Error {
    /// The type of this error.
    pub kind: ErrorKind,
    /// A description of this error.
    pub message: String,
}

pub(crate) fn maybe<T>(result: Result<T, Error>) -> Result<Option<T>, Error> {
    match result {
        Ok(val) => Ok(Some(val)),
        Err(e) => {
            if e.kind == ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(&format!("{:?}: {}", self.kind, self.message))
    }
}

impl From<Error> for String {
    fn from(error: Error) -> String {
        format!("{}", error)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        let kind = if let Some(status) = error.status() {
            if status == StatusCode::NOT_FOUND {
                ErrorKind::NotFound
            } else if status == StatusCode::UNAUTHORIZED {
                ErrorKind::NotAuthenticated
            } else if status.is_server_error() {
                ErrorKind::Server
            } else {
                ErrorKind::Client
            }
        } else {
            ErrorKind::Network
        };

        Self {
            kind,
            message: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self {
            kind: ErrorKind::Response,
            message: error.to_string(),
        }
    }
}
