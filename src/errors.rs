#[derive(Debug)]
pub enum LicenseError {
    NetworkError(String),
    InvalidLicense,
    ServerError(String),
}

impl std::fmt::Display for LicenseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for LicenseError {}

// Implement conversion from reqwest::Error to LicenseError
impl From<reqwest::Error> for LicenseError {
    fn from(err: reqwest::Error) -> Self {
        LicenseError::NetworkError(err.to_string())
    }
}
