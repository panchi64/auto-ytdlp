use std::fmt;
use std::result;
use std::sync::{MutexGuard, PoisonError};
use thiserror::Error;

pub type Result<T> = result::Result<T, AppError>;

/// Application-wide error enum
///
/// Note: Some variants may appear unused in the current implementation but are
/// retained for future use and to provide a complete error type hierarchy.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to access concurrent resource: {0}")]
    Lock(String),

    // Intentionally retained for future network-related error handling
    #[allow(dead_code)]
    #[error("Network error: {0}")]
    Network(String),

    // Intentionally retained for future download process error handling
    #[allow(dead_code)]
    #[error("Download error: {0}")]
    Download(String),

    // Intentionally retained for future dependency checking
    #[allow(dead_code)]
    #[error("Missing dependency: {0}")]
    Dependency(String),

    // Intentionally retained for future configuration validation
    #[allow(dead_code)]
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Clipboard error: {0}")]
    Clipboard(String),

    // Intentionally retained for future UI error handling
    #[allow(dead_code)]
    #[error("UI error: {0}")]
    Ui(String),

    #[error("Channel communication error: {0}")]
    Channel(String),

    #[error("{0}")]
    Other(String),
}

impl<T> From<PoisonError<MutexGuard<'_, T>>> for AppError {
    fn from(err: PoisonError<MutexGuard<'_, T>>) -> Self {
        AppError::Lock(err.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Other(err.to_string())
    }
}

/// Extension traits for working with Results and Options//
///
/// These utility traits provide convenience methods for error handling.
/// While some methods may not be currently used, they are provided for
// Extension trait for Option to easily convert to AppError
pub trait OptionExt<T> {
    /// Converts an Option to a Result with a custom AppError
    ///
    /// This method is currently unused but retained for future use.
    #[allow(dead_code)]
    fn ok_or_app_err<F>(self, err_fn: F) -> Result<T>
    where
        F: FnOnce() -> AppError;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_app_err<F>(self, err_fn: F) -> Result<T>
    where
        F: FnOnce() -> AppError,
    {
        self.ok_or_else(err_fn)
    }
}

// Extension trait for Result to easily map errors
pub trait ResultExt<T, E> {
    /// Maps an error from one type to AppError using a mapping function
    ///
    /// This method is currently unused but retained for future use.
    #[allow(dead_code)]
    fn map_app_err<F>(self, err_fn: F) -> Result<T>
    where
        F: FnOnce(E) -> AppError;
}

impl<T, E: fmt::Debug> ResultExt<T, E> for result::Result<T, E> {
    fn map_app_err<F>(self, err_fn: F) -> Result<T>
    where
        F: FnOnce(E) -> AppError,
    {
        self.map_err(err_fn)
    }
}
