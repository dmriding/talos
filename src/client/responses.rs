//! Response types for the Talos license client API.
//!
//! These types represent successful responses from the license server
//! and provide structured access to license information.

use serde::{Deserialize, Serialize};

/// Result of a successful license validation.
///
/// Returned by `License::validate()` and `License::validate_offline()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// List of features enabled for this license
    pub features: Vec<String>,

    /// License tier name (e.g., "pro", "enterprise")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,

    /// License expiration date (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,

    /// For air-gapped systems: when the offline grace period ends
    ///
    /// If this is present, the system must connect to the license server
    /// before this time to maintain validity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_ends_at: Option<String>,

    /// Warning message from the server (e.g., approaching expiration)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

impl ValidationResult {
    /// Returns true if a grace period warning is present.
    pub fn has_grace_period_warning(&self) -> bool {
        self.grace_period_ends_at.is_some()
    }

    /// Returns true if any warning is present.
    pub fn has_warning(&self) -> bool {
        self.warning.is_some()
    }

    /// Check if a specific feature is enabled.
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }
}

/// Result of a successful license bind operation.
///
/// Returned by `License::bind()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindResult {
    /// Server-side license ID (UUID)
    pub license_id: String,

    /// List of features enabled for this license
    pub features: Vec<String>,

    /// License tier name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,

    /// License expiration date (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

impl BindResult {
    /// Check if a specific feature is enabled.
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }
}

/// Result of a feature validation check.
///
/// Returned by `License::validate_feature()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureResult {
    /// Whether the feature is allowed for this license
    pub allowed: bool,

    /// Optional message explaining the result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// License tier name (for context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
}

/// Result of a heartbeat operation.
///
/// Returned by `License::heartbeat()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResult {
    /// Server timestamp (RFC 3339 format)
    pub server_time: String,

    /// Updated grace period end time (for air-gapped systems)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_ends_at: Option<String>,
}

// === Server Response Parsing ===
// These types match the server's JSON response format for deserialization.
// Some fields are required for proper JSON deserialization but may not be
// directly used after parsing.

/// Server response for bind endpoint.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct ServerBindResponse {
    pub success: bool,
    pub license_id: String,
    pub features: Vec<String>,
    pub tier: Option<String>,
    pub expires_at: Option<String>,
}

impl From<ServerBindResponse> for BindResult {
    fn from(resp: ServerBindResponse) -> Self {
        Self {
            license_id: resp.license_id,
            features: resp.features,
            tier: resp.tier,
            expires_at: resp.expires_at,
        }
    }
}

/// Server response for release endpoint.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct ServerReleaseResponse {
    pub success: bool,
    pub message: String,
}

/// Server response for validate endpoint.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct ServerValidateResponse {
    pub valid: bool,
    pub license_id: Option<String>,
    pub features: Option<Vec<String>>,
    pub tier: Option<String>,
    pub expires_at: Option<String>,
    pub grace_period_ends_at: Option<String>,
    pub warning: Option<String>,
}

impl From<ServerValidateResponse> for ValidationResult {
    fn from(resp: ServerValidateResponse) -> Self {
        Self {
            features: resp.features.unwrap_or_default(),
            tier: resp.tier,
            expires_at: resp.expires_at,
            grace_period_ends_at: resp.grace_period_ends_at,
            warning: resp.warning,
        }
    }
}

/// Server response for heartbeat endpoint.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct ServerHeartbeatResponse {
    pub success: bool,
    pub server_time: String,
}

impl From<ServerHeartbeatResponse> for HeartbeatResult {
    fn from(resp: ServerHeartbeatResponse) -> Self {
        Self {
            server_time: resp.server_time,
            // Server doesn't currently return this, but we'll support it for future
            grace_period_ends_at: None,
        }
    }
}

/// Server response for validate-feature endpoint.
#[derive(Debug, Deserialize)]
pub(crate) struct ServerFeatureResponse {
    pub allowed: bool,
    pub message: Option<String>,
    pub tier: Option<String>,
}

impl From<ServerFeatureResponse> for FeatureResult {
    fn from(resp: ServerFeatureResponse) -> Self {
        Self {
            allowed: resp.allowed,
            message: resp.message,
            tier: resp.tier,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_result_has_feature() {
        let result = ValidationResult {
            features: vec!["feature_a".to_string(), "feature_b".to_string()],
            tier: Some("pro".to_string()),
            expires_at: None,
            grace_period_ends_at: None,
            warning: None,
        };

        assert!(result.has_feature("feature_a"));
        assert!(result.has_feature("feature_b"));
        assert!(!result.has_feature("feature_c"));
    }

    #[test]
    fn validation_result_grace_period_warning() {
        let with_grace = ValidationResult {
            features: vec![],
            tier: None,
            expires_at: None,
            grace_period_ends_at: Some("2024-12-31T23:59:59Z".to_string()),
            warning: Some("Must connect by 2024-12-31".to_string()),
        };

        assert!(with_grace.has_grace_period_warning());
        assert!(with_grace.has_warning());

        let without_grace = ValidationResult {
            features: vec![],
            tier: None,
            expires_at: None,
            grace_period_ends_at: None,
            warning: None,
        };

        assert!(!without_grace.has_grace_period_warning());
        assert!(!without_grace.has_warning());
    }

    #[test]
    fn parse_server_bind_response() {
        let json = r#"{
            "success": true,
            "license_id": "550e8400-e29b-41d4-a716-446655440000",
            "features": ["feature_a", "feature_b"],
            "tier": "pro",
            "expires_at": "2025-12-31T23:59:59Z"
        }"#;

        let resp: ServerBindResponse = serde_json::from_str(json).unwrap();
        let result: BindResult = resp.into();

        assert_eq!(result.license_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(result.features.len(), 2);
        assert_eq!(result.tier, Some("pro".to_string()));
    }

    #[test]
    fn parse_server_validate_response() {
        let json = r#"{
            "valid": true,
            "license_id": "550e8400-e29b-41d4-a716-446655440000",
            "features": ["feature_a"],
            "tier": "enterprise",
            "expires_at": "2025-12-31T23:59:59Z",
            "grace_period_ends_at": "2025-01-15T12:00:00Z",
            "warning": "System must connect by January 15"
        }"#;

        let resp: ServerValidateResponse = serde_json::from_str(json).unwrap();
        let result: ValidationResult = resp.into();

        assert!(result.has_feature("feature_a"));
        assert!(result.has_grace_period_warning());
        assert!(result.has_warning());
    }
}
