use std::io;
use std::error::Error;
use std::string::FromUtf8Error;

/// Error kinds
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ErrorKind {
    /// invalid server response
    ResponseError,
    /// The authentication with server failed
    AuthenticationFailed,
    /// Operation failed because of a type mismatch
    TypeError,
    /// A script execution was aborted
    ExecAbortError,
    /// The server can't response because it's busy
    BusyLoadingError,
    /// A script that was requested does not actually exists
    NoScriptError,
    /// A error that is unknown to the library.
    ExtensionError(String),
    /// IoError
    IoError
}

/// Redis Value Enum
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Value {
    /// nil response
    Nil,
    /// integer response
    Int(i64),
    /// an arbiary binary data
    Data(Vec<u8>),
    /// nested structures response
    Bulk(Vec<Value>),
    /// a status response, normally a string
    Status(String),
    /// "OK" response
    Okay,    
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct RedisError {
    pub kind: ErrorKind,
    pub desc: &'static str,
    pub detail: Option<String>,
}

impl From<io::Error> for RedisError {
    fn from(err: io::Error) -> RedisError {
        RedisError {
            kind: ErrorKind::IoError,
            desc: "An internal IO error occurred",
            detail: Some(err.description().to_string())
        }
    }
}

impl From<FromUtf8Error> for RedisError {
    fn from(err: FromUtf8Error) -> RedisError {
        RedisError {
            kind: ErrorKind::TypeError,
            desc: "Invalid UTF-8",
            detail: Some(err.description().to_string())
        }
    }
}

impl From<(ErrorKind, &'static str)> for RedisError {
    fn from((kind, desc): (ErrorKind, &'static str)) -> RedisError {
        RedisError {
            kind: kind,
            desc: desc,
            detail: None,
        }
    }
}

impl From<(ErrorKind, &'static str, String)> for RedisError {
    fn from((kind, desc, detail): (ErrorKind, &'static str, String)) -> RedisError {
        RedisError {
            kind: kind,
            desc: desc,
            detail: Some(detail),
        }
    }
}

pub type RedisResult<T> = Result<T, RedisError>;
