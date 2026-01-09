//! Standardized API error responses for all Talos endpoints.
//!
//! This module provides a unified error response format across all API endpoints:
//! - Legacy client endpoints (`/activate`, `/validate`, etc.)
//! - Client API v1 (`/api/v1/client/*`)
//! - Admin API (`/api/v1/licenses/*`, `/api/v1/tokens/*`)
//!
//! # Response Format
//!
//! All error responses follow this JSON structure:
//!
//! ```json
//! {
//!   "error": {
//!     "code": "LICENSE_NOT_FOUND",
//!     "message": "The requested license does not exist",
//!     "details": null
//!   }
//! }
//! ```
//!
//! The `details` field is optional and may contain additional context.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::errors::LicenseError;

/// Machine-readable error codes for API responses.
///
/// These codes are stable and can be used by clients for programmatic error handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // === License State Errors (4xx) ===
    /// License key was not found in the database
    LicenseNotFound,
    /// License has expired
    LicenseExpired,
    /// License has been revoked
    LicenseRevoked,
    /// License is suspended (temporary)
    LicenseSuspended,
    /// License has been permanently blacklisted
    LicenseBlacklisted,
    /// License exists but is not in active state
    LicenseInactive,

    // === Hardware Binding Errors (4xx) ===
    /// License is already bound to a different device
    AlreadyBound,
    /// License is not bound to any device
    NotBound,
    /// Request hardware ID doesn't match bound device
    HardwareMismatch,

    // === Feature/Quota Errors (4xx) ===
    /// Requested feature is not included in license tier
    FeatureNotIncluded,
    /// Usage quota has been exceeded
    QuotaExceeded,

    // === Validation Errors (400) ===
    /// Request payload is invalid or malformed
    InvalidRequest,
    /// A required field is missing
    MissingField,
    /// A field value is invalid
    InvalidField,

    // === Authentication Errors (401/403) ===
    /// No authentication token provided
    MissingToken,
    /// Authorization header is malformed
    InvalidHeader,
    /// Authentication token is invalid
    InvalidToken,
    /// Authentication token has expired
    TokenExpired,
    /// Token lacks required permissions
    InsufficientScope,
    /// Authentication is not configured on server
    AuthDisabled,

    // === Resource Errors (404/409) ===
    /// Requested resource was not found
    NotFound,
    /// Operation conflicts with current state
    Conflict,

    // === Server Errors (5xx) ===
    /// Database operation failed
    DatabaseError,
    /// Server configuration error
    ConfigError,
    /// Encryption/decryption operation failed
    CryptoError,
    /// External service communication failed
    NetworkError,
    /// Unexpected internal server error
    InternalError,
}

impl ErrorCode {
    /// Returns the HTTP status code for this error code.
    pub fn status_code(&self) -> StatusCode {
        match self {
            // 400 Bad Request
            ErrorCode::InvalidRequest
            | ErrorCode::MissingField
            | ErrorCode::InvalidField
            | ErrorCode::InvalidHeader => StatusCode::BAD_REQUEST,

            // 401 Unauthorized
            ErrorCode::MissingToken | ErrorCode::InvalidToken | ErrorCode::TokenExpired => {
                StatusCode::UNAUTHORIZED
            }

            // 403 Forbidden
            ErrorCode::LicenseExpired
            | ErrorCode::LicenseRevoked
            | ErrorCode::LicenseSuspended
            | ErrorCode::LicenseBlacklisted
            | ErrorCode::LicenseInactive
            | ErrorCode::HardwareMismatch
            | ErrorCode::FeatureNotIncluded
            | ErrorCode::QuotaExceeded
            | ErrorCode::InsufficientScope => StatusCode::FORBIDDEN,

            // 404 Not Found
            ErrorCode::LicenseNotFound | ErrorCode::NotFound => StatusCode::NOT_FOUND,

            // 409 Conflict
            ErrorCode::AlreadyBound | ErrorCode::NotBound | ErrorCode::Conflict => {
                StatusCode::CONFLICT
            }

            // 500 Internal Server Error
            ErrorCode::DatabaseError
            | ErrorCode::ConfigError
            | ErrorCode::CryptoError
            | ErrorCode::InternalError => StatusCode::INTERNAL_SERVER_ERROR,

            // 501 Not Implemented
            ErrorCode::AuthDisabled => StatusCode::NOT_IMPLEMENTED,

            // 502 Bad Gateway
            ErrorCode::NetworkError => StatusCode::BAD_GATEWAY,
        }
    }

    /// Returns a default human-readable message for this error code.
    pub fn default_message(&self) -> &'static str {
        match self {
            ErrorCode::LicenseNotFound => "The requested license does not exist",
            ErrorCode::LicenseExpired => "License has expired",
            ErrorCode::LicenseRevoked => "License has been revoked",
            ErrorCode::LicenseSuspended => "License is temporarily suspended",
            ErrorCode::LicenseBlacklisted => "License has been permanently blacklisted",
            ErrorCode::LicenseInactive => "License is not active",
            ErrorCode::AlreadyBound => "License is already bound to another device",
            ErrorCode::NotBound => "License is not bound to any device",
            ErrorCode::HardwareMismatch => "Hardware ID does not match the bound device",
            ErrorCode::FeatureNotIncluded => "Feature is not included in your license tier",
            ErrorCode::QuotaExceeded => "Usage quota has been exceeded",
            ErrorCode::InvalidRequest => "Request payload is invalid",
            ErrorCode::MissingField => "A required field is missing",
            ErrorCode::InvalidField => "A field value is invalid",
            ErrorCode::MissingToken => "Authentication token is required",
            ErrorCode::InvalidHeader => "Authorization header is malformed",
            ErrorCode::InvalidToken => "Authentication token is invalid",
            ErrorCode::TokenExpired => "Authentication token has expired",
            ErrorCode::InsufficientScope => "Insufficient permissions for this operation",
            ErrorCode::AuthDisabled => "Authentication is not configured on this server",
            ErrorCode::NotFound => "The requested resource was not found",
            ErrorCode::Conflict => "Operation conflicts with current resource state",
            ErrorCode::DatabaseError => "Database operation failed",
            ErrorCode::ConfigError => "Server configuration error",
            ErrorCode::CryptoError => "Encryption operation failed",
            ErrorCode::NetworkError => "Failed to communicate with external service",
            ErrorCode::InternalError => "An unexpected error occurred",
        }
    }
}

/// The inner error object containing code, message, and optional details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ErrorBody {
    /// Machine-readable error code
    pub code: ErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Optional additional details (field name, constraint violated, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Standardized API error response.
///
/// This is the top-level error response returned by all API endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ApiError {
    /// The error details
    pub error: ErrorBody,
}

impl ApiError {
    /// Creates a new API error with the given code.
    ///
    /// Uses the default message for the error code.
    pub fn new(code: ErrorCode) -> Self {
        Self {
            error: ErrorBody {
                code,
                message: code.default_message().to_string(),
                details: None,
            },
        }
    }

    /// Creates a new API error with a custom message.
    pub fn with_message(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            error: ErrorBody {
                code,
                message: message.into(),
                details: None,
            },
        }
    }

    /// Creates a new API error with a custom message and details.
    pub fn with_details(
        code: ErrorCode,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            error: ErrorBody {
                code,
                message: message.into(),
                details: Some(details),
            },
        }
    }

    /// Adds details to an existing error.
    pub fn details(mut self, details: serde_json::Value) -> Self {
        self.error.details = Some(details);
        self
    }

    /// Returns the HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        self.error.code.status_code()
    }

    // === Convenience constructors for common errors ===

    /// License not found error.
    pub fn license_not_found() -> Self {
        Self::new(ErrorCode::LicenseNotFound)
    }

    /// License not found with key in message.
    pub fn license_not_found_key(key: &str) -> Self {
        Self::with_message(
            ErrorCode::LicenseNotFound,
            format!("License '{}' not found", key),
        )
    }

    /// Invalid request error with field details.
    pub fn invalid_field(field: &str, reason: &str) -> Self {
        Self::with_details(
            ErrorCode::InvalidField,
            format!("Invalid value for '{}': {}", field, reason),
            serde_json::json!({ "field": field }),
        )
    }

    /// Missing required field error.
    pub fn missing_field(field: &str) -> Self {
        Self::with_details(
            ErrorCode::MissingField,
            format!("Required field '{}' is missing", field),
            serde_json::json!({ "field": field }),
        )
    }

    /// Resource not found error.
    pub fn not_found(resource: &str) -> Self {
        Self::with_message(ErrorCode::NotFound, format!("{} not found", resource))
    }

    /// Database error (internal details hidden from client).
    pub fn database_error() -> Self {
        Self::new(ErrorCode::DatabaseError)
    }

    /// Internal server error.
    pub fn internal_error() -> Self {
        Self::new(ErrorCode::InternalError)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        (status, Json(self)).into_response()
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {}",
            self.error.code.default_message(),
            self.error.message
        )
    }
}

impl std::error::Error for ApiError {}

// === Conversions from existing error types ===

impl From<LicenseError> for ApiError {
    fn from(err: LicenseError) -> Self {
        match err {
            LicenseError::InvalidLicense(msg) => {
                ApiError::with_message(ErrorCode::InvalidRequest, msg)
            }
            LicenseError::ConfigError(msg) => ApiError::with_message(ErrorCode::ConfigError, msg),
            LicenseError::NetworkError(e) => {
                ApiError::with_message(ErrorCode::NetworkError, e.to_string())
            }
            LicenseError::StorageError(e) => {
                ApiError::with_message(ErrorCode::InternalError, e.to_string())
            }
            LicenseError::EncryptionError(msg)
            | LicenseError::DecryptionError(msg)
            | LicenseError::KeyringError(msg) => {
                ApiError::with_message(ErrorCode::CryptoError, msg)
            }
            LicenseError::ServerError(msg) => ApiError::with_message(ErrorCode::InternalError, msg),
            LicenseError::UnknownError => ApiError::new(ErrorCode::InternalError),
            LicenseError::ClientApiError(e) => {
                // Map client error codes to server error codes
                use crate::client::errors::ClientErrorCode;
                let code = match e.code {
                    ClientErrorCode::LicenseNotFound => ErrorCode::LicenseNotFound,
                    ClientErrorCode::LicenseExpired => ErrorCode::LicenseExpired,
                    ClientErrorCode::LicenseRevoked => ErrorCode::LicenseRevoked,
                    ClientErrorCode::LicenseSuspended => ErrorCode::LicenseSuspended,
                    ClientErrorCode::LicenseBlacklisted => ErrorCode::LicenseBlacklisted,
                    ClientErrorCode::LicenseInactive => ErrorCode::LicenseInactive,
                    ClientErrorCode::AlreadyBound => ErrorCode::AlreadyBound,
                    ClientErrorCode::NotBound => ErrorCode::NotBound,
                    ClientErrorCode::HardwareMismatch => ErrorCode::HardwareMismatch,
                    ClientErrorCode::FeatureNotIncluded => ErrorCode::FeatureNotIncluded,
                    ClientErrorCode::QuotaExceeded => ErrorCode::QuotaExceeded,
                    ClientErrorCode::GracePeriodExpired
                    | ClientErrorCode::InternalError
                    | ClientErrorCode::Unknown => ErrorCode::InternalError,
                };
                ApiError::with_message(code, e.message)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_status_mapping() {
        assert_eq!(
            ErrorCode::LicenseNotFound.status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ErrorCode::InvalidRequest.status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ErrorCode::MissingToken.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ErrorCode::LicenseExpired.status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(ErrorCode::AlreadyBound.status_code(), StatusCode::CONFLICT);
        assert_eq!(
            ErrorCode::DatabaseError.status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn api_error_serialization() {
        let err = ApiError::license_not_found();
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("LICENSE_NOT_FOUND"));
        assert!(json.contains("message"));
    }

    #[test]
    fn api_error_with_details() {
        let err = ApiError::invalid_field("email", "must be a valid email address");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("INVALID_FIELD"));
        assert!(json.contains("email"));
    }

    #[test]
    fn license_error_conversion() {
        let license_err = LicenseError::InvalidLicense("bad key".to_string());
        let api_err: ApiError = license_err.into();
        assert_eq!(api_err.error.code, ErrorCode::InvalidRequest);
    }
}
