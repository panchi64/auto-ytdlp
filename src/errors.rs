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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn test_error_display_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let app_err: AppError = io_err.into();
        assert!(app_err.to_string().contains("IO error"));
        assert!(app_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_error_display_lock() {
        let err = AppError::Lock("mutex poisoned".to_string());
        assert_eq!(
            err.to_string(),
            "Failed to access concurrent resource: mutex poisoned"
        );
    }

    #[test]
    fn test_error_display_clipboard() {
        let err = AppError::Clipboard("clipboard unavailable".to_string());
        assert_eq!(err.to_string(), "Clipboard error: clipboard unavailable");
    }

    #[test]
    fn test_error_display_channel() {
        let err = AppError::Channel("channel closed".to_string());
        assert_eq!(
            err.to_string(),
            "Channel communication error: channel closed"
        );
    }

    #[test]
    fn test_error_display_other() {
        let err = AppError::Other("something went wrong".to_string());
        assert_eq!(err.to_string(), "something went wrong");
    }

    #[test]
    fn test_poison_error_conversion() {
        // Create a poisoned mutex by panicking while holding it
        let mutex = Mutex::new(42);
        let result = std::panic::catch_unwind(|| {
            let _guard = mutex.lock().unwrap();
            panic!("intentional panic to poison mutex");
        });
        assert!(result.is_err());

        // Now try to acquire the poisoned lock
        let poison_err = mutex.lock().unwrap_err();
        let app_err: AppError = poison_err.into();

        // Verify it converts to AppError::Lock
        match app_err {
            AppError::Lock(msg) => {
                assert!(msg.contains("poisoned"));
            }
            _ => panic!("Expected AppError::Lock variant"),
        }
    }

    #[test]
    fn test_anyhow_error_conversion() {
        let anyhow_err = anyhow::anyhow!("an anyhow error occurred");
        let app_err: AppError = anyhow_err.into();

        match app_err {
            AppError::Other(msg) => {
                assert!(msg.contains("an anyhow error occurred"));
            }
            _ => panic!("Expected AppError::Other variant"),
        }
    }
}
