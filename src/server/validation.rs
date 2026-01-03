//! Request validation utilities for Talos API.
//!
//! This module provides validation functions for common input types
//! used across the API endpoints.

use std::fmt;

/// Validation error type.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Result type for validation operations.
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Validate a UUID format.
///
/// Accepts UUIDs in the standard format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
///
/// # Example
/// ```
/// use talos::server::validation::validate_uuid;
///
/// assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000", "license_id").is_ok());
/// assert!(validate_uuid("invalid-uuid", "license_id").is_err());
/// ```
pub fn validate_uuid(value: &str, field_name: &str) -> ValidationResult<()> {
    // UUID pattern: 8-4-4-4-12 hex chars
    let uuid_regex = regex::Regex::new(
        r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
    )
    .unwrap();

    if uuid_regex.is_match(value) {
        Ok(())
    } else {
        Err(ValidationError {
            field: field_name.to_string(),
            message: "invalid UUID format (expected: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)"
                .to_string(),
        })
    }
}

/// Validate a license key format.
///
/// License keys follow the pattern: `PREFIX-XXXX-XXXX-XXXX-XXXX` where:
/// - PREFIX is 2-10 uppercase alphanumeric characters
/// - Each segment is 2-6 uppercase alphanumeric characters (excluding ambiguous: 0, O, I, L, 1)
///
/// # Example
/// ```
/// use talos::server::validation::validate_license_key;
///
/// assert!(validate_license_key("LIC-ABCD-EFGH-IJKL", "license_key").is_ok());
/// assert!(validate_license_key("KERYX-A2B3-C4D5-E6F7-G8H9", "license_key").is_ok());
/// assert!(validate_license_key("invalid", "license_key").is_err());
/// ```
pub fn validate_license_key(value: &str, field_name: &str) -> ValidationResult<()> {
    // License key pattern: PREFIX followed by 2-5 segments of alphanumeric chars
    // More flexible to support various configurations
    let key_regex = regex::Regex::new(r"^[A-Z0-9]{2,10}(-[A-Z2-9]{2,6}){2,5}$").unwrap();

    if key_regex.is_match(value) {
        Ok(())
    } else {
        Err(ValidationError {
            field: field_name.to_string(),
            message: "invalid license key format (expected: PREFIX-XXXX-XXXX-XXXX)".to_string(),
        })
    }
}

/// Validate a hardware ID format.
///
/// Hardware IDs are SHA-256 hashes represented as 64 hexadecimal characters.
///
/// # Example
/// ```
/// use talos::server::validation::validate_hardware_id;
///
/// let valid_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
/// assert!(validate_hardware_id(valid_hash, "hardware_id").is_ok());
/// assert!(validate_hardware_id("invalid", "hardware_id").is_err());
/// ```
pub fn validate_hardware_id(value: &str, field_name: &str) -> ValidationResult<()> {
    // SHA-256 hash: exactly 64 hex characters
    let hex_regex = regex::Regex::new(r"^[0-9a-fA-F]{64}$").unwrap();

    if hex_regex.is_match(value) {
        Ok(())
    } else {
        Err(ValidationError {
            field: field_name.to_string(),
            message: "invalid hardware ID format (expected: 64 hex characters, SHA-256 hash)"
                .to_string(),
        })
    }
}

/// Validate that a string is not empty or whitespace only.
///
/// # Example
/// ```
/// use talos::server::validation::validate_not_empty;
///
/// assert!(validate_not_empty("hello", "name").is_ok());
/// assert!(validate_not_empty("", "name").is_err());
/// assert!(validate_not_empty("   ", "name").is_err());
/// ```
pub fn validate_not_empty(value: &str, field_name: &str) -> ValidationResult<()> {
    if value.trim().is_empty() {
        Err(ValidationError {
            field: field_name.to_string(),
            message: "cannot be empty".to_string(),
        })
    } else {
        Ok(())
    }
}

/// Validate string length is within bounds.
///
/// # Example
/// ```
/// use talos::server::validation::validate_length;
///
/// assert!(validate_length("hello", 1, 10, "name").is_ok());
/// assert!(validate_length("", 1, 10, "name").is_err());
/// assert!(validate_length("a".repeat(100).as_str(), 1, 10, "name").is_err());
/// ```
pub fn validate_length(
    value: &str,
    min: usize,
    max: usize,
    field_name: &str,
) -> ValidationResult<()> {
    let len = value.len();
    if len < min {
        Err(ValidationError {
            field: field_name.to_string(),
            message: format!("must be at least {} characters", min),
        })
    } else if len > max {
        Err(ValidationError {
            field: field_name.to_string(),
            message: format!("must be at most {} characters", max),
        })
    } else {
        Ok(())
    }
}

/// Validate an optional string - if present, validates it's not empty.
pub fn validate_optional_not_empty(value: Option<&str>, field_name: &str) -> ValidationResult<()> {
    if let Some(v) = value {
        validate_not_empty(v, field_name)
    } else {
        Ok(())
    }
}

/// Validate an ISO 8601 datetime string.
///
/// Accepts formats:
/// - RFC 3339: `2025-12-31T23:59:59Z`
/// - Date only: `2025-12-31`
/// - Without timezone: `2025-12-31T23:59:59`
pub fn validate_datetime(value: &str, field_name: &str) -> ValidationResult<()> {
    // Try RFC 3339
    if chrono::DateTime::parse_from_rfc3339(value).is_ok() {
        return Ok(());
    }

    // Try date only
    if chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok() {
        return Ok(());
    }

    // Try datetime without timezone
    if chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").is_ok() {
        return Ok(());
    }

    Err(ValidationError {
        field: field_name.to_string(),
        message: "invalid datetime format (expected: ISO 8601, e.g., '2025-12-31T23:59:59Z' or '2025-12-31')".to_string(),
    })
}

/// Validate a feature name.
///
/// Feature names should be alphanumeric with underscores/hyphens, 1-64 chars.
pub fn validate_feature_name(value: &str, field_name: &str) -> ValidationResult<()> {
    let feature_regex = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]{0,63}$").unwrap();

    if feature_regex.is_match(value) {
        Ok(())
    } else {
        Err(ValidationError {
            field: field_name.to_string(),
            message:
                "invalid feature name (must start with letter, contain only alphanumeric, underscore, hyphen, max 64 chars)"
                    .to_string(),
        })
    }
}

/// Validate an organization ID.
///
/// Org IDs are flexible - alphanumeric with hyphens/underscores, 1-128 chars.
pub fn validate_org_id(value: &str, field_name: &str) -> ValidationResult<()> {
    let org_regex = regex::Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9_-]{0,127}$").unwrap();

    if org_regex.is_match(value) {
        Ok(())
    } else {
        Err(ValidationError {
            field: field_name.to_string(),
            message: "invalid org_id format (alphanumeric with hyphens/underscores, 1-128 chars)"
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_uuid_valid() {
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000", "id").is_ok());
        assert!(validate_uuid("00000000-0000-0000-0000-000000000000", "id").is_ok());
        assert!(validate_uuid("FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF", "id").is_ok());
    }

    #[test]
    fn test_validate_uuid_invalid() {
        assert!(validate_uuid("invalid", "id").is_err());
        assert!(validate_uuid("550e8400-e29b-41d4-a716", "id").is_err());
        assert!(validate_uuid("550e8400e29b41d4a716446655440000", "id").is_err());
        assert!(validate_uuid("", "id").is_err());
    }

    #[test]
    fn test_validate_license_key_valid() {
        assert!(validate_license_key("LIC-ABCD-EFGH-IJKL", "key").is_ok());
        assert!(validate_license_key("KERYX-A2B3-C4D5-E6F7-G8H9", "key").is_ok());
        assert!(validate_license_key("AB-CD-EF-GH", "key").is_ok());
    }

    #[test]
    fn test_validate_license_key_invalid() {
        assert!(validate_license_key("invalid", "key").is_err());
        assert!(validate_license_key("ABC", "key").is_err());
        assert!(validate_license_key("", "key").is_err());
    }

    #[test]
    fn test_validate_hardware_id_valid() {
        let valid_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert!(validate_hardware_id(valid_hash, "hw").is_ok());

        let another_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(validate_hardware_id(another_hash, "hw").is_ok());
    }

    #[test]
    fn test_validate_hardware_id_invalid() {
        assert!(validate_hardware_id("invalid", "hw").is_err());
        assert!(validate_hardware_id("e3b0c44298fc1c149afbf4c8996fb924", "hw").is_err()); // 32 chars
        assert!(validate_hardware_id("", "hw").is_err());
        // Contains non-hex character 'g'
        assert!(validate_hardware_id(
            "g3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "hw"
        )
        .is_err());
    }

    #[test]
    fn test_validate_not_empty() {
        assert!(validate_not_empty("hello", "field").is_ok());
        assert!(validate_not_empty("a", "field").is_ok());
        assert!(validate_not_empty("", "field").is_err());
        assert!(validate_not_empty("   ", "field").is_err());
        assert!(validate_not_empty("\t\n", "field").is_err());
    }

    #[test]
    fn test_validate_length() {
        assert!(validate_length("hello", 1, 10, "field").is_ok());
        assert!(validate_length("a", 1, 10, "field").is_ok());
        assert!(validate_length("", 1, 10, "field").is_err());
        assert!(validate_length("hello world", 1, 10, "field").is_err());
        assert!(validate_length("hello", 10, 20, "field").is_err());
    }

    #[test]
    fn test_validate_datetime() {
        assert!(validate_datetime("2025-12-31T23:59:59Z", "dt").is_ok());
        assert!(validate_datetime("2025-12-31T23:59:59+00:00", "dt").is_ok());
        assert!(validate_datetime("2025-12-31", "dt").is_ok());
        assert!(validate_datetime("2025-12-31T23:59:59", "dt").is_ok());
        assert!(validate_datetime("invalid", "dt").is_err());
        assert!(validate_datetime("31-12-2025", "dt").is_err());
    }

    #[test]
    fn test_validate_feature_name() {
        assert!(validate_feature_name("feature_a", "feat").is_ok());
        assert!(validate_feature_name("relay", "feat").is_ok());
        assert!(validate_feature_name("premium-feature", "feat").is_ok());
        assert!(validate_feature_name("Feature123", "feat").is_ok());
        assert!(validate_feature_name("_invalid", "feat").is_err()); // starts with underscore
        assert!(validate_feature_name("123abc", "feat").is_err()); // starts with number
        assert!(validate_feature_name("", "feat").is_err());
    }

    #[test]
    fn test_validate_org_id() {
        assert!(validate_org_id("org-123", "org").is_ok());
        assert!(validate_org_id("my_organization", "org").is_ok());
        assert!(validate_org_id("Company123", "org").is_ok());
        assert!(validate_org_id("a", "org").is_ok());
        assert!(validate_org_id("-invalid", "org").is_err()); // starts with hyphen
        assert!(validate_org_id("", "org").is_err());
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError {
            field: "test_field".to_string(),
            message: "is invalid".to_string(),
        };
        assert_eq!(err.to_string(), "test_field: is invalid");
    }
}
