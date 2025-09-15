use std::fmt;
use std::io;

#[derive(Debug)]
pub enum AppError {
    Io(io::Error),
    Server(String),
    Configuration(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(err) => write!(f, "IO error: {err}"),
            AppError::Server(msg) => write!(f, "Server error: {msg}"),
            AppError::Configuration(msg) => write!(f, "Configuration error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::Io(err)
    }
}

impl From<axum::Error> for AppError {
    fn from(err: axum::Error) -> Self {
        AppError::Server(err.to_string())
    }
}

impl From<std::num::ParseIntError> for AppError {
    fn from(err: std::num::ParseIntError) -> Self {
        AppError::Configuration(format!("Failed to parse port number: {err}"))
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub trait ContextExt<T> {
    fn with_context<C, F>(self, f: F) -> Result<T, AppError>
    where
        F: FnOnce() -> C,
        C: fmt::Display;
}

impl<T> ContextExt<T> for Result<T, io::Error> {
    fn with_context<C, F>(self, f: F) -> Result<T, AppError>
    where
        F: FnOnce() -> C,
        C: fmt::Display,
    {
        self.map_err(|err| AppError::Server(format!("{}: {}", f(), err)))
    }
}
