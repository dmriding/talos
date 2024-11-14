use std::fmt;
use std::error::Error;

/// Custom error type for license-related operations
#[derive(Debug)]
pub enum LicenseError {
    ServerError(String),
    InvalidLicense(String),      // Renamed from LicenseInvalid to InvalidLicense
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
            LicenseError::InvalidLicense(msg) => write!(f, "License Invalid: {}", msg),  // Updated to match the new name
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

/// Implement From for reqwest::Error
impl From<reqwest::Error> for LicenseError {
    fn from(error: reqwest::Error) -> Self {
        LicenseError::ServerError(format!("Reqwest error: {}", error))
    }
}

/// Implement From for std::io::Error
impl From<std::io::Error> for LicenseError {
    fn from(error: std::io::Error) -> Self {
        LicenseError::StorageError(format!("IO error: {}", error))
    }
}
