use std::result;
use std::sync::{MutexGuard, PoisonError};
use thiserror::Error;

pub type Result<T> = result::Result<T, AppError>;

/// Application-wide error enum
#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to access concurrent resource: {0}")]
    Lock(String),

    #[error("Clipboard error: {0}")]
    Clipboard(String),

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
