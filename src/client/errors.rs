//! Client-side error types for the Talos license client.
//!
//! This module provides error types that match the server's API error responses,
//! allowing clients to handle specific error conditions programmatically.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Error codes returned by the Talos license server API.
///
/// These codes correspond to the server's `ClientErrorCode` enum and allow
/// clients to handle specific error conditions without parsing error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClientErrorCode {
    // === License State Errors ===
    /// License key was not found in the database
    LicenseNotFound,
    /// License has expired
    LicenseExpired,
    /// License has been revoked
    LicenseRevoked,
    /// License is suspended (may have grace period)
    LicenseSuspended,
    /// License has been permanently blacklisted
    LicenseBlacklisted,
    /// License exists but is not in active state
    LicenseInactive,

    // === Hardware Binding Errors ===
    /// License is already bound to a different device
    AlreadyBound,
    /// License is not bound to any device
    NotBound,
    /// Request hardware ID doesn't match bound device
    HardwareMismatch,

    // === Feature/Quota Errors ===
    /// Requested feature is not included in license tier
    FeatureNotIncluded,
    /// Usage quota has been exceeded
    QuotaExceeded,

    // === Grace Period Errors (client-side) ===
    /// Cached grace period has expired, must go online
    GracePeriodExpired,

    // === Server Errors ===
    /// Internal server error
    InternalError,

    // === Unknown ===
    /// Unknown error code (forward compatibility)
    #[serde(other)]
    Unknown,
}

impl ClientErrorCode {
    /// Returns a default human-readable message for this error code.
    pub fn default_message(&self) -> &'static str {
        match self {
            ClientErrorCode::LicenseNotFound => "License not found",
            ClientErrorCode::LicenseExpired => "License has expired",
            ClientErrorCode::LicenseRevoked => "License has been revoked",
            ClientErrorCode::LicenseSuspended => "License is suspended",
            ClientErrorCode::LicenseBlacklisted => "License has been blacklisted",
            ClientErrorCode::LicenseInactive => "License is not active",
            ClientErrorCode::AlreadyBound => "License is already bound to another device",
            ClientErrorCode::NotBound => "License is not bound to any device",
            ClientErrorCode::HardwareMismatch => "Hardware ID does not match",
            ClientErrorCode::FeatureNotIncluded => "Feature not included in license",
            ClientErrorCode::QuotaExceeded => "Usage quota exceeded",
            ClientErrorCode::GracePeriodExpired => {
                "Grace period expired - please connect to license server"
            }
            ClientErrorCode::InternalError => "Internal server error",
            ClientErrorCode::Unknown => "Unknown error",
        }
    }

    /// Returns true if this error indicates the license is invalid and cannot be used.
    pub fn is_license_invalid(&self) -> bool {
        matches!(
            self,
            ClientErrorCode::LicenseNotFound
                | ClientErrorCode::LicenseExpired
                | ClientErrorCode::LicenseRevoked
                | ClientErrorCode::LicenseBlacklisted
                | ClientErrorCode::GracePeriodExpired
        )
    }

    /// Returns true if this error might be resolved by going online.
    pub fn requires_online(&self) -> bool {
        matches!(
            self,
            ClientErrorCode::GracePeriodExpired | ClientErrorCode::LicenseSuspended
        )
    }
}

impl fmt::Display for ClientErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.default_message())
    }
}

/// The inner error body from a server API error response.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerErrorBody {
    /// Machine-readable error code
    pub code: ClientErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Optional additional details
    #[serde(default)]
    pub details: Option<serde_json::Value>,
}

/// Server API error response wrapper.
///
/// Matches the server's `ApiError` JSON structure:
/// ```json
/// {
///   "error": {
///     "code": "LICENSE_NOT_FOUND",
///     "message": "The requested license does not exist",
///     "details": null
///   }
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct ServerErrorResponse {
    pub error: ServerErrorBody,
}

/// Error returned when a license server API call fails.
///
/// This wraps the server's error response and provides convenient access
/// to the error code for programmatic handling.
#[derive(Debug, Clone)]
pub struct ClientApiError {
    /// Machine-readable error code
    pub code: ClientErrorCode,
    /// Human-readable error message from server
    pub message: String,
    /// Optional additional details (e.g., field name for validation errors)
    pub details: Option<serde_json::Value>,
}

impl ClientApiError {
    /// Create a new client API error.
    pub fn new(code: ClientErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    /// Create a new client API error with details.
    pub fn with_details(
        code: ClientErrorCode,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            details: Some(details),
        }
    }

    /// Create an error for grace period expiration (client-side only).
    pub fn grace_period_expired() -> Self {
        Self::new(
            ClientErrorCode::GracePeriodExpired,
            "Offline grace period has expired. Please connect to the license server.",
        )
    }

    /// Returns true if this error indicates the license is invalid.
    pub fn is_license_invalid(&self) -> bool {
        self.code.is_license_invalid()
    }

    /// Returns true if this error might be resolved by going online.
    pub fn requires_online(&self) -> bool {
        self.code.requires_online()
    }
}

impl fmt::Display for ClientApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ClientApiError {}

impl From<ServerErrorResponse> for ClientApiError {
    fn from(resp: ServerErrorResponse) -> Self {
        Self {
            code: resp.error.code,
            message: resp.error.message,
            details: resp.error.details,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_server_error_response() {
        let json = r#"{
            "error": {
                "code": "LICENSE_NOT_FOUND",
                "message": "The requested license does not exist",
                "details": null
            }
        }"#;

        let resp: ServerErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.error.code, ClientErrorCode::LicenseNotFound);
        assert_eq!(resp.error.message, "The requested license does not exist");
        assert!(resp.error.details.is_none());
    }

    #[test]
    fn parse_already_bound_error() {
        let json = r#"{
            "error": {
                "code": "ALREADY_BOUND",
                "message": "License is already bound to device 'Work Laptop'",
                "details": {"device_name": "Work Laptop"}
            }
        }"#;

        let resp: ServerErrorResponse = serde_json::from_str(json).unwrap();
        let err: ClientApiError = resp.into();

        assert_eq!(err.code, ClientErrorCode::AlreadyBound);
        assert!(err.message.contains("Work Laptop"));
        assert!(err.details.is_some());
    }

    #[test]
    fn parse_unknown_error_code() {
        let json = r#"{
            "error": {
                "code": "SOME_FUTURE_ERROR",
                "message": "Some new error type",
                "details": null
            }
        }"#;

        let resp: ServerErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.error.code, ClientErrorCode::Unknown);
    }

    #[test]
    fn error_code_is_license_invalid() {
        assert!(ClientErrorCode::LicenseNotFound.is_license_invalid());
        assert!(ClientErrorCode::LicenseExpired.is_license_invalid());
        assert!(ClientErrorCode::LicenseRevoked.is_license_invalid());
        assert!(ClientErrorCode::GracePeriodExpired.is_license_invalid());

        assert!(!ClientErrorCode::AlreadyBound.is_license_invalid());
        assert!(!ClientErrorCode::NotBound.is_license_invalid());
        assert!(!ClientErrorCode::LicenseSuspended.is_license_invalid());
    }

    #[test]
    fn error_code_requires_online() {
        assert!(ClientErrorCode::GracePeriodExpired.requires_online());
        assert!(ClientErrorCode::LicenseSuspended.requires_online());

        assert!(!ClientErrorCode::LicenseExpired.requires_online());
        assert!(!ClientErrorCode::AlreadyBound.requires_online());
    }

    #[test]
    fn client_api_error_display() {
        let err = ClientApiError::new(
            ClientErrorCode::LicenseExpired,
            "Your license expired on 2024-01-01",
        );
        let display = format!("{}", err);
        assert!(display.contains("expired"));
    }
}
