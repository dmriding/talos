//! Client API endpoints for license binding and validation.
//!
//! These endpoints are used by client applications to bind, release, and validate licenses.
//! They do not require authentication but are rate-limited to prevent abuse.
//!
//! # Endpoints
//!
//! - `POST /api/v1/client/bind` - Bind a license to hardware
//! - `POST /api/v1/client/release` - Release a license from hardware
//! - `POST /api/v1/client/validate` - Validate a license
//! - `POST /api/v1/client/validate-or-bind` - Validate or auto-bind a license
//! - `POST /api/v1/client/heartbeat` - Send heartbeat ping

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::server::database::{BindingAction, PerformedBy};
use crate::server::handlers::AppState;

/// Error codes for client API responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClientErrorCode {
    /// License key not found
    LicenseNotFound,
    /// License is already bound to different hardware
    AlreadyBound,
    /// License is not bound (for release operations)
    NotBound,
    /// Hardware ID doesn't match the bound hardware
    HardwareMismatch,
    /// License is expired
    LicenseExpired,
    /// License is revoked
    LicenseRevoked,
    /// License is suspended (may be in grace period)
    LicenseSuspended,
    /// License is blacklisted
    LicenseBlacklisted,
    /// License is not active
    LicenseInactive,
    /// Invalid request format
    InvalidRequest,
    /// Internal server error
    InternalError,
}

/// Client API error response.
#[derive(Debug, Serialize)]
pub struct ClientError {
    pub success: bool,
    pub error: ClientErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bound_device: Option<String>,
}

impl ClientError {
    pub fn new(code: ClientErrorCode, message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: code,
            message: message.into(),
            bound_device: None,
        }
    }

    pub fn with_bound_device(mut self, device: Option<String>) -> Self {
        self.bound_device = device;
        self
    }

    pub fn status_code(&self) -> StatusCode {
        match self.error {
            ClientErrorCode::LicenseNotFound => StatusCode::NOT_FOUND,
            ClientErrorCode::AlreadyBound => StatusCode::CONFLICT,
            ClientErrorCode::NotBound => StatusCode::CONFLICT,
            ClientErrorCode::HardwareMismatch => StatusCode::FORBIDDEN,
            ClientErrorCode::LicenseExpired => StatusCode::FORBIDDEN,
            ClientErrorCode::LicenseRevoked => StatusCode::FORBIDDEN,
            ClientErrorCode::LicenseSuspended => StatusCode::FORBIDDEN,
            ClientErrorCode::LicenseBlacklisted => StatusCode::FORBIDDEN,
            ClientErrorCode::LicenseInactive => StatusCode::FORBIDDEN,
            ClientErrorCode::InvalidRequest => StatusCode::BAD_REQUEST,
            ClientErrorCode::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ClientError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        (status, Json(self)).into_response()
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request to bind a license to hardware.
#[derive(Debug, Deserialize)]
pub struct BindRequest {
    /// The human-readable license key (e.g., "LIC-XXXX-XXXX-XXXX")
    pub license_key: String,
    /// Hardware fingerprint (SHA-256 hash)
    pub hardware_id: String,
    /// Optional device name for display purposes
    #[serde(default)]
    pub device_name: Option<String>,
    /// Optional device info (OS, CPU, etc.)
    #[serde(default)]
    pub device_info: Option<String>,
}

/// Response from a successful bind operation.
#[derive(Debug, Serialize)]
pub struct BindResponse {
    pub success: bool,
    pub license_id: String,
    pub features: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Request to release a license from hardware.
#[derive(Debug, Deserialize)]
pub struct ReleaseRequest {
    /// The human-readable license key
    pub license_key: String,
    /// Hardware fingerprint to verify ownership
    pub hardware_id: String,
}

/// Response from a release operation.
#[derive(Debug, Serialize)]
pub struct ReleaseResponse {
    pub success: bool,
    pub message: String,
}

/// Request to validate a license.
#[derive(Debug, Deserialize)]
pub struct ValidateRequest {
    /// The human-readable license key
    pub license_key: String,
    /// Hardware fingerprint to verify binding
    pub hardware_id: String,
}

/// Response from validation.
#[derive(Debug, Serialize)]
pub struct ValidateResponse {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_ends_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// Request for validate-or-bind operation.
#[derive(Debug, Deserialize)]
pub struct ValidateOrBindRequest {
    /// The human-readable license key
    pub license_key: String,
    /// Hardware fingerprint
    pub hardware_id: String,
    /// Optional device name (used if binding)
    #[serde(default)]
    pub device_name: Option<String>,
    /// Optional device info (used if binding)
    #[serde(default)]
    pub device_info: Option<String>,
}

/// Request for heartbeat.
#[derive(Debug, Deserialize)]
pub struct ClientHeartbeatRequest {
    /// The human-readable license key
    pub license_key: String,
    /// Hardware fingerprint to verify binding
    pub hardware_id: String,
}

/// Response from heartbeat.
#[derive(Debug, Serialize)]
pub struct ClientHeartbeatResponse {
    pub success: bool,
    pub server_time: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// Bind a license to hardware.
///
/// # Behavior
/// - Checks if license exists and is valid
/// - If unbound, binds to the provided hardware
/// - If already bound to same hardware, returns success
/// - If bound to different hardware, returns ALREADY_BOUND error
pub async fn bind_handler(
    State(state): State<AppState>,
    Json(req): Json<BindRequest>,
) -> Result<Json<BindResponse>, ClientError> {
    info!("Bind request for license_key={}", req.license_key);

    // Find the license
    let license = state
        .db
        .get_license_by_key(&req.license_key)
        .await
        .map_err(|e| {
            warn!("Database error: {}", e);
            ClientError::new(ClientErrorCode::InternalError, "Database error")
        })?
        .ok_or_else(|| {
            warn!("License not found: {}", req.license_key);
            ClientError::new(ClientErrorCode::LicenseNotFound, "License key not found")
        })?;

    // Check license status
    if license.is_blacklisted == Some(true) {
        return Err(ClientError::new(
            ClientErrorCode::LicenseBlacklisted,
            "License is blacklisted",
        ));
    }
    if license.status == "revoked" {
        return Err(ClientError::new(
            ClientErrorCode::LicenseRevoked,
            "License has been revoked",
        ));
    }
    if license.status == "suspended" && !license.is_in_grace_period() {
        return Err(ClientError::new(
            ClientErrorCode::LicenseSuspended,
            "License is suspended",
        ));
    }
    if license.status != "active" && license.status != "suspended" {
        return Err(ClientError::new(
            ClientErrorCode::LicenseInactive,
            format!("License status is '{}'", license.status),
        ));
    }
    if license.is_expired() {
        return Err(ClientError::new(
            ClientErrorCode::LicenseExpired,
            "License has expired",
        ));
    }

    // Check if already bound
    if license.is_bound() {
        if license.hardware_id.as_deref() == Some(&req.hardware_id) {
            // Already bound to this hardware - return success
            info!("License {} already bound to this hardware", req.license_key);
            return Ok(Json(BindResponse {
                success: true,
                license_id: license.license_id,
                features: parse_features(&license.features),
                tier: license.tier,
                expires_at: license.expires_at.map(|d| d.to_string()),
            }));
        } else {
            // Bound to different hardware
            return Err(ClientError::new(
                ClientErrorCode::AlreadyBound,
                "License is already bound to a different device",
            )
            .with_bound_device(license.device_name));
        }
    }

    // Bind the license
    state
        .db
        .bind_license(
            &license.license_id,
            &req.hardware_id,
            req.device_name.as_deref(),
            req.device_info.as_deref(),
        )
        .await
        .map_err(|e| {
            warn!("Failed to bind license: {}", e);
            ClientError::new(ClientErrorCode::InternalError, "Failed to bind license")
        })?;

    // Record binding history
    let _ = state
        .db
        .record_binding_history(
            &license.license_id,
            BindingAction::Bind,
            Some(&req.hardware_id),
            req.device_name.as_deref(),
            req.device_info.as_deref(),
            PerformedBy::Client,
            None,
        )
        .await;

    info!(
        "License {} bound to hardware {}",
        req.license_key, req.hardware_id
    );

    Ok(Json(BindResponse {
        success: true,
        license_id: license.license_id,
        features: parse_features(&license.features),
        tier: license.tier,
        expires_at: license.expires_at.map(|d| d.to_string()),
    }))
}

/// Release a license from hardware.
///
/// # Behavior
/// - Verifies hardware_id matches the bound hardware
/// - Clears hardware binding fields
/// - Records release in binding history
pub async fn release_handler(
    State(state): State<AppState>,
    Json(req): Json<ReleaseRequest>,
) -> Result<Json<ReleaseResponse>, ClientError> {
    info!("Release request for license_key={}", req.license_key);

    // Find the license
    let license = state
        .db
        .get_license_by_key(&req.license_key)
        .await
        .map_err(|e| {
            warn!("Database error: {}", e);
            ClientError::new(ClientErrorCode::InternalError, "Database error")
        })?
        .ok_or_else(|| {
            warn!("License not found: {}", req.license_key);
            ClientError::new(ClientErrorCode::LicenseNotFound, "License key not found")
        })?;

    // Check if bound
    if !license.is_bound() {
        return Err(ClientError::new(
            ClientErrorCode::NotBound,
            "License is not currently bound",
        ));
    }

    // Verify hardware ID matches
    if license.hardware_id.as_deref() != Some(&req.hardware_id) {
        return Err(ClientError::new(
            ClientErrorCode::HardwareMismatch,
            "Hardware ID does not match the bound device",
        ));
    }

    // Release the license
    state
        .db
        .release_license(&license.license_id)
        .await
        .map_err(|e| {
            warn!("Failed to release license: {}", e);
            ClientError::new(ClientErrorCode::InternalError, "Failed to release license")
        })?;

    // Record release in history
    let _ = state
        .db
        .record_binding_history(
            &license.license_id,
            BindingAction::Release,
            Some(&req.hardware_id),
            license.device_name.as_deref(),
            license.device_info.as_deref(),
            PerformedBy::Client,
            None,
        )
        .await;

    info!(
        "License {} released from hardware {}",
        req.license_key, req.hardware_id
    );

    Ok(Json(ReleaseResponse {
        success: true,
        message: "License released successfully".to_string(),
    }))
}

/// Validate a license.
///
/// # Behavior
/// - Checks license exists
/// - Checks license is not expired, revoked, suspended, or blacklisted
/// - Checks license is bound to the provided hardware
/// - Updates last_seen_at timestamp
/// - Returns license details including features and tier
pub async fn validate_handler(
    State(state): State<AppState>,
    Json(req): Json<ValidateRequest>,
) -> Result<Json<ValidateResponse>, ClientError> {
    info!("Validate request for license_key={}", req.license_key);

    // Find the license
    let license = state
        .db
        .get_license_by_key(&req.license_key)
        .await
        .map_err(|e| {
            warn!("Database error: {}", e);
            ClientError::new(ClientErrorCode::InternalError, "Database error")
        })?
        .ok_or_else(|| {
            warn!("License not found: {}", req.license_key);
            ClientError::new(ClientErrorCode::LicenseNotFound, "License key not found")
        })?;

    // Check if blacklisted
    if license.is_blacklisted == Some(true) {
        return Err(ClientError::new(
            ClientErrorCode::LicenseBlacklisted,
            "License is blacklisted",
        ));
    }

    // Check if revoked
    if license.status == "revoked" {
        return Err(ClientError::new(
            ClientErrorCode::LicenseRevoked,
            "License has been revoked",
        ));
    }

    // Check if expired
    if license.is_expired() {
        return Err(ClientError::new(
            ClientErrorCode::LicenseExpired,
            "License has expired",
        ));
    }

    // Check if bound
    if !license.is_bound() {
        return Err(ClientError::new(
            ClientErrorCode::NotBound,
            "License is not bound to any device",
        ));
    }

    // Check hardware ID matches
    if license.hardware_id.as_deref() != Some(&req.hardware_id) {
        return Err(ClientError::new(
            ClientErrorCode::HardwareMismatch,
            "Hardware ID does not match the bound device",
        ));
    }

    // Update last_seen_at
    let _ = state.db.update_last_seen(&license.license_id).await;

    // Handle suspended status (grace period) - check before building response
    let (grace_period_ends, warning_msg) = if license.status == "suspended" {
        if license.is_in_grace_period() {
            (
                license.grace_period_ends_at.map(|d| d.to_string()),
                Some(
                    license
                        .suspension_message
                        .clone()
                        .unwrap_or_else(|| "License is in grace period".to_string()),
                ),
            )
        } else {
            return Err(ClientError::new(
                ClientErrorCode::LicenseSuspended,
                "License is suspended and grace period has ended",
            ));
        }
    } else {
        (None, None)
    };

    // Check for non-active status
    if license.status != "active" && license.status != "suspended" {
        return Err(ClientError::new(
            ClientErrorCode::LicenseInactive,
            format!("License status is '{}'", license.status),
        ));
    }

    // Build response
    let response = ValidateResponse {
        valid: true,
        license_id: Some(license.license_id),
        features: Some(parse_features(&license.features)),
        tier: license.tier,
        expires_at: license.expires_at.map(|d| d.to_string()),
        grace_period_ends_at: grace_period_ends,
        warning: warning_msg,
    };

    info!("License {} validated successfully", req.license_key);

    Ok(Json(response))
}

/// Validate or bind a license.
///
/// # Behavior
/// - If bound to this hardware: validate and return
/// - If unbound: bind first, then validate
/// - If bound to other hardware: return ALREADY_BOUND error
pub async fn validate_or_bind_handler(
    State(state): State<AppState>,
    Json(req): Json<ValidateOrBindRequest>,
) -> Result<Json<ValidateResponse>, ClientError> {
    info!(
        "Validate-or-bind request for license_key={}",
        req.license_key
    );

    // Find the license
    let license = state
        .db
        .get_license_by_key(&req.license_key)
        .await
        .map_err(|e| {
            warn!("Database error: {}", e);
            ClientError::new(ClientErrorCode::InternalError, "Database error")
        })?
        .ok_or_else(|| {
            warn!("License not found: {}", req.license_key);
            ClientError::new(ClientErrorCode::LicenseNotFound, "License key not found")
        })?;

    // Check license validity first
    if license.is_blacklisted == Some(true) {
        return Err(ClientError::new(
            ClientErrorCode::LicenseBlacklisted,
            "License is blacklisted",
        ));
    }
    if license.status == "revoked" {
        return Err(ClientError::new(
            ClientErrorCode::LicenseRevoked,
            "License has been revoked",
        ));
    }
    if license.is_expired() {
        return Err(ClientError::new(
            ClientErrorCode::LicenseExpired,
            "License has expired",
        ));
    }
    if license.status == "suspended" && !license.is_in_grace_period() {
        return Err(ClientError::new(
            ClientErrorCode::LicenseSuspended,
            "License is suspended",
        ));
    }
    if license.status != "active" && license.status != "suspended" {
        return Err(ClientError::new(
            ClientErrorCode::LicenseInactive,
            format!("License status is '{}'", license.status),
        ));
    }

    // Check binding status
    if license.is_bound() {
        if license.hardware_id.as_deref() != Some(&req.hardware_id) {
            // Bound to different hardware
            return Err(ClientError::new(
                ClientErrorCode::AlreadyBound,
                "License is already bound to a different device",
            )
            .with_bound_device(license.device_name));
        }
        // Already bound to this hardware - just validate
    } else {
        // Not bound - bind it first
        state
            .db
            .bind_license(
                &license.license_id,
                &req.hardware_id,
                req.device_name.as_deref(),
                req.device_info.as_deref(),
            )
            .await
            .map_err(|e| {
                warn!("Failed to bind license: {}", e);
                ClientError::new(ClientErrorCode::InternalError, "Failed to bind license")
            })?;

        // Record binding history
        let _ = state
            .db
            .record_binding_history(
                &license.license_id,
                BindingAction::Bind,
                Some(&req.hardware_id),
                req.device_name.as_deref(),
                req.device_info.as_deref(),
                PerformedBy::Client,
                None,
            )
            .await;

        info!(
            "License {} auto-bound to hardware {}",
            req.license_key, req.hardware_id
        );
    }

    // Update last_seen_at
    let _ = state.db.update_last_seen(&license.license_id).await;

    // Handle grace period warning - check before building response
    let (grace_period_ends, warning_msg) =
        if license.status == "suspended" && license.is_in_grace_period() {
            (
                license.grace_period_ends_at.map(|d| d.to_string()),
                Some(
                    license
                        .suspension_message
                        .clone()
                        .unwrap_or_else(|| "License is in grace period".to_string()),
                ),
            )
        } else {
            (None, None)
        };

    // Build response
    let response = ValidateResponse {
        valid: true,
        license_id: Some(license.license_id),
        features: Some(parse_features(&license.features)),
        tier: license.tier,
        expires_at: license.expires_at.map(|d| d.to_string()),
        grace_period_ends_at: grace_period_ends,
        warning: warning_msg,
    };

    Ok(Json(response))
}

/// Client heartbeat endpoint using license key.
///
/// # Behavior
/// - Verifies license exists and is bound to the provided hardware
/// - Updates last_seen_at timestamp
/// - Returns server timestamp
pub async fn client_heartbeat_handler(
    State(state): State<AppState>,
    Json(req): Json<ClientHeartbeatRequest>,
) -> Result<Json<ClientHeartbeatResponse>, ClientError> {
    info!("Heartbeat for license_key={}", req.license_key);

    // Find the license
    let license = state
        .db
        .get_license_by_key(&req.license_key)
        .await
        .map_err(|e| {
            warn!("Database error: {}", e);
            ClientError::new(ClientErrorCode::InternalError, "Database error")
        })?
        .ok_or_else(|| {
            warn!("License not found: {}", req.license_key);
            ClientError::new(ClientErrorCode::LicenseNotFound, "License key not found")
        })?;

    // Check if bound
    if !license.is_bound() {
        return Err(ClientError::new(
            ClientErrorCode::NotBound,
            "License is not bound to any device",
        ));
    }

    // Verify hardware ID matches
    if license.hardware_id.as_deref() != Some(&req.hardware_id) {
        return Err(ClientError::new(
            ClientErrorCode::HardwareMismatch,
            "Hardware ID does not match the bound device",
        ));
    }

    // Update last_seen_at
    state
        .db
        .update_last_seen(&license.license_id)
        .await
        .map_err(|e| {
            warn!("Failed to update last_seen: {}", e);
            ClientError::new(ClientErrorCode::InternalError, "Failed to update heartbeat")
        })?;

    Ok(Json(ClientHeartbeatResponse {
        success: true,
        server_time: Utc::now().to_rfc3339(),
    }))
}

// ============================================================================
// Helpers
// ============================================================================

/// Parse features from JSON string to Vec<String>.
fn parse_features(features: &Option<String>) -> Vec<String> {
    features
        .as_ref()
        .and_then(|f| serde_json::from_str::<Vec<String>>(f).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_serialization() {
        let err = ClientError::new(ClientErrorCode::LicenseNotFound, "Not found");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("LICENSE_NOT_FOUND"));
    }

    #[test]
    fn error_status_codes() {
        assert_eq!(
            ClientError::new(ClientErrorCode::LicenseNotFound, "").status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ClientError::new(ClientErrorCode::AlreadyBound, "").status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            ClientError::new(ClientErrorCode::HardwareMismatch, "").status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            ClientError::new(ClientErrorCode::InvalidRequest, "").status_code(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn parse_features_empty() {
        assert_eq!(parse_features(&None), Vec::<String>::new());
        assert_eq!(parse_features(&Some("".to_string())), Vec::<String>::new());
    }

    #[test]
    fn parse_features_valid() {
        let features = Some(r#"["feature_a", "feature_b"]"#.to_string());
        assert_eq!(
            parse_features(&features),
            vec!["feature_a".to_string(), "feature_b".to_string()]
        );
    }
}
