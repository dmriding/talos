use std::fmt;
use std::error::Error;

/// Custom error type for license-related operations
#[derive(Debug)]
pub enum LicenseError {
    ServerError(String),
    LicenseInvalid(String),
    NetworkError(String),
    StorageError(String),
    EncryptionError(String),
    DecryptionError(String),
    ConfigError(String),
    UnknownError,
}

impl fmt::Display for LicenseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LicenseError::ServerError(msg) => write!(f, "Server Error: {}", msg),
            LicenseError::LicenseInvalid(msg) => write!(f, "License Invalid: {}", msg),
            LicenseError::NetworkError(msg) => write!(f, "Network Error: {}", msg),
            LicenseError::StorageError(msg) => write!(f, "Storage Error: {}", msg),
            LicenseError::EncryptionError(msg) => write!(f, "Encryption Error: {}", msg),
            LicenseError::DecryptionError(msg) => write!(f, "Decryption Error: {}", msg),
            LicenseError::ConfigError(msg) => write!(f, "Config Error: {}", msg),
            LicenseError::UnknownError => write!(f, "Unknown Error"),
        }
    }
}

impl Error for LicenseError {}
