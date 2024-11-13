use std::fmt;

/// Enum representing different types of license errors.
#[derive(Debug)]
pub enum LicenseError {
    NetworkError(String),
    InvalidLicense,
    ServerError(String),
    EncryptionError(String),
    DecryptionError(String),
    HardwareError(String),
}

impl fmt::Display for LicenseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LicenseError::NetworkError(msg) => write!(f, "Network Error: {}", msg),
            LicenseError::InvalidLicense => write!(f, "Invalid License"),
            LicenseError::ServerError(msg) => write!(f, "Server Error: {}", msg),
            LicenseError::EncryptionError(msg) => write!(f, "Encryption Error: {}", msg),
            LicenseError::DecryptionError(msg) => write!(f, "Decryption Error: {}", msg),
            LicenseError::HardwareError(msg) => write!(f, "Hardware Error: {}", msg),
        }
    }
}

impl std::error::Error for LicenseError {}

// Implement conversion from reqwest::Error to LicenseError
impl From<reqwest::Error> for LicenseError {
    fn from(err: reqwest::Error) -> Self {
        LicenseError::NetworkError(err.to_string())
    }
}

// Implement conversion from std::io::Error to LicenseError
impl From<std::io::Error> for LicenseError {
    fn from(err: std::io::Error) -> Self {
        LicenseError::ServerError(err.to_string())
    }
}

// Implement conversion from ring::error::Unspecified to LicenseError
impl From<ring::error::Unspecified> for LicenseError {
    fn from(_: ring::error::Unspecified) -> Self {
        LicenseError::EncryptionError("Unspecified error".to_string())
    }
}
