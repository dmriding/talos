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
