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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_app_error_display() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let app_error = AppError::Io(io_error);
        assert!(app_error.to_string().contains("IO error"));
        assert!(app_error.to_string().contains("file not found"));
    }

    #[test]
    fn test_server_error_display() {
        let app_error = AppError::Server("connection failed".to_string());
        assert_eq!(app_error.to_string(), "Server error: connection failed");
    }

    #[test]
    fn test_configuration_error_display() {
        let app_error = AppError::Configuration("invalid port".to_string());
        assert_eq!(app_error.to_string(), "Configuration error: invalid port");
    }

    #[test]
    fn test_from_io_error() {
        let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        let app_error: AppError = io_error.into();
        match app_error {
            AppError::Io(_) => assert!(true),
            _ => assert!(false, "Expected Io variant"),
        }
    }

    #[test]
    fn test_from_parse_int_error() {
        let parse_error = "abc".parse::<i32>().unwrap_err();
        let app_error: AppError = parse_error.into();
        match app_error {
            AppError::Configuration(msg) => {
                assert!(msg.contains("Failed to parse port number"));
            }
            _ => assert!(false, "Expected Configuration variant"),
        }
    }

    #[test]
    fn test_context_ext_with_context() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let result: Result<String, io::Error> = Err(io_error);

        let context_result = result.with_context(|| "Failed to read config");
        assert!(context_result.is_err());

        match context_result {
            Err(AppError::Server(msg)) => {
                assert!(msg.contains("Failed to read config"));
                assert!(msg.contains("file not found"));
            }
            _ => assert!(false, "Expected Server variant with context"),
        }
    }

    #[test]
    fn test_context_ext_success() {
        let result: Result<String, io::Error> = Ok("success".to_string());
        let context_result = result.with_context(|| "This should not appear");
        assert!(context_result.is_ok());
        assert_eq!(context_result.unwrap(), "success");
    }

    #[test]
    fn test_app_result_type() {
        let success: AppResult<String> = Ok("test".to_string());
        assert!(success.is_ok());

        let failure: AppResult<String> = Err(AppError::Server("error".to_string()));
        assert!(failure.is_err());
    }
}
