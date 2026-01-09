// src/errors.rs

use std::result;

use thiserror::Error;

use crate::client::errors::ClientApiError;

/// Convenient alias for results throughout Talos.
pub type LicenseResult<T> = result::Result<T, LicenseError>;

/// Central error type for license-related operations.
///
/// This is used by both the client and the server side of Talos.
/// HTTP mapping / printing / logging should be done *outside* this type.
#[derive(Debug, Error)]
pub enum LicenseError {
    /// The server responded with an application-level error.
    #[error("server error: {0}")]
    ServerError(String),

    /// The license is invalid for the requested operation.
    #[error("license invalid: {0}")]
    InvalidLicense(String),

    /// Network / HTTP client errors when talking to the licensing server.
    #[error("network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    /// Local storage errors (filesystem, OS I/O, etc.).
    #[error("storage error: {0}")]
    StorageError(#[from] std::io::Error),

    /// Errors during encryption (wrong key, algorithm failure, etc.).
    #[error("encryption error: {0}")]
    EncryptionError(String),

    /// Errors during decryption (corrupted ciphertext, wrong key, etc.).
    #[error("decryption error: {0}")]
    DecryptionError(String),

    /// Errors when accessing the OS keyring/credential store.
    #[error("keyring error: {0}")]
    KeyringError(String),

    /// Configuration-related errors (missing values, invalid formats, etc.).
    #[error("config error: {0}")]
    ConfigError(String),

    /// Structured API error from the license server.
    ///
    /// This wraps the server's error response with machine-readable error codes
    /// for programmatic error handling.
    #[error("api error: {0}")]
    ClientApiError(ClientApiError),

    /// Fallback for unexpected conditions that don't fit other variants.
    #[error("unknown error")]
    UnknownError,
}
