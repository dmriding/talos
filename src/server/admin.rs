//! Admin API handlers for license management.
//!
//! This module provides CRUD operations for licenses, intended for admin use.
//! All endpoints require JWT authentication when the `jwt-auth` feature is enabled.
//!
//! # Endpoints
//!
//! - `POST /api/v1/licenses` - Create a new license
//! - `POST /api/v1/licenses/batch` - Create multiple licenses
//! - `GET /api/v1/licenses/{license_id}` - Get a license by ID
//! - `GET /api/v1/licenses?org_id={id}` - List licenses for an organization
//! - `PATCH /api/v1/licenses/{license_id}` - Update a license
//! - `POST /api/v1/licenses/{license_id}/release` - Force release from hardware
//! - `POST /api/v1/licenses/{license_id}/revoke` - Revoke a license
//! - `POST /api/v1/licenses/{license_id}/reinstate` - Reinstate a revoked/suspended license
//! - `POST /api/v1/licenses/{license_id}/extend` - Extend license expiration
//! - `PATCH /api/v1/licenses/{license_id}/usage` - Update bandwidth/usage tracking

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{NaiveDateTime, Utc};

// Import traits for test assertions
#[cfg(test)]
use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::config::get_config;
use crate::errors::{LicenseError, LicenseResult};
use crate::license_key::{generate_license_key, LicenseKeyConfig};
use crate::server::database::{Database, License};
use crate::server::handlers::AppState;
use crate::tiers::get_tier_features;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request body for creating a new license.
#[derive(Debug, Deserialize)]
pub struct CreateLicenseRequest {
    /// Organization ID (optional)
    pub org_id: Option<String>,
    /// Organization name (optional)
    pub org_name: Option<String>,
    /// Tier name - if provided and tiers are configured, features are derived from tier
    pub tier: Option<String>,
    /// Features to enable - if tier is provided, these are merged with tier features
    #[serde(default)]
    pub features: Vec<String>,
    /// Expiration date (ISO 8601 format: "2025-12-31T23:59:59")
    pub expires_at: Option<String>,
    /// Additional metadata as JSON
    pub metadata: Option<serde_json::Value>,
}

/// Request body for batch creating licenses.
#[derive(Debug, Deserialize)]
pub struct BatchCreateLicenseRequest {
    /// Number of licenses to create
    pub count: u32,
    /// Organization ID (optional, applied to all)
    pub org_id: Option<String>,
    /// Organization name (optional, applied to all)
    pub org_name: Option<String>,
    /// Tier name (optional, applied to all)
    pub tier: Option<String>,
    /// Features (optional, applied to all)
    #[serde(default)]
    pub features: Vec<String>,
    /// Expiration date (optional, applied to all)
    pub expires_at: Option<String>,
}

/// Request body for updating a license.
#[derive(Debug, Deserialize)]
pub struct UpdateLicenseRequest {
    /// New tier (re-derives features if tiers configured)
    pub tier: Option<String>,
    /// New features (replaces existing)
    pub features: Option<Vec<String>>,
    /// New expiration date
    pub expires_at: Option<String>,
    /// New metadata
    pub metadata: Option<serde_json::Value>,
}

/// Query parameters for listing licenses.
#[derive(Debug, Deserialize)]
pub struct ListLicensesQuery {
    /// Filter by organization ID
    pub org_id: Option<String>,
    /// Pagination: page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Pagination: items per page
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 {
    1
}
fn default_per_page() -> u32 {
    50
}

/// Response for a single license.
#[derive(Debug, Serialize)]
pub struct LicenseResponse {
    pub license_id: String,
    pub license_key: Option<String>,
    pub status: String,
    pub org_id: Option<String>,
    pub org_name: Option<String>,
    pub tier: Option<String>,
    pub features: Vec<String>,
    pub issued_at: String,
    pub expires_at: Option<String>,
    pub is_bound: bool,
    pub hardware_id: Option<String>,
    pub device_name: Option<String>,
    pub bound_at: Option<String>,
    pub last_seen_at: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl From<License> for LicenseResponse {
    fn from(license: License) -> Self {
        let features: Vec<String> = license
            .features
            .as_ref()
            .map(|f| serde_json::from_str(f).unwrap_or_default())
            .unwrap_or_default();

        let metadata: Option<serde_json::Value> = license
            .metadata
            .as_ref()
            .and_then(|m| serde_json::from_str(m).ok());

        let is_bound = license.is_bound();

        Self {
            license_id: license.license_id,
            license_key: license.license_key,
            status: license.status,
            org_id: license.org_id,
            org_name: license.org_name,
            tier: license.tier,
            features,
            issued_at: license.issued_at.to_string(),
            expires_at: license.expires_at.map(|d| d.to_string()),
            is_bound,
            hardware_id: license.hardware_id,
            device_name: license.device_name,
            bound_at: license.bound_at.map(|d| d.to_string()),
            last_seen_at: license.last_seen_at.map(|d| d.to_string()),
            metadata,
        }
    }
}

/// Response for batch create operation.
#[derive(Debug, Serialize)]
pub struct BatchCreateResponse {
    pub created: u32,
    pub licenses: Vec<LicenseSummary>,
}

/// Summary of a created license (for batch operations).
#[derive(Debug, Serialize)]
pub struct LicenseSummary {
    pub license_id: String,
    pub license_key: String,
}

/// Response for listing licenses.
#[derive(Debug, Serialize)]
pub struct ListLicensesResponse {
    pub licenses: Vec<LicenseResponse>,
    pub total: u32,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

/// Admin API error type.
#[derive(Debug)]
pub enum AdminError {
    /// License not found
    NotFound(String),
    /// Invalid request data
    BadRequest(String),
    /// Database error
    DatabaseError(String),
    /// Configuration error
    ConfigError(String),
}

impl std::fmt::Display for AdminError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdminError::NotFound(msg) => write!(f, "not found: {msg}"),
            AdminError::BadRequest(msg) => write!(f, "bad request: {msg}"),
            AdminError::DatabaseError(msg) => write!(f, "database error: {msg}"),
            AdminError::ConfigError(msg) => write!(f, "configuration error: {msg}"),
        }
    }
}

impl std::error::Error for AdminError {}

impl IntoResponse for AdminError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AdminError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            AdminError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            AdminError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR"),
            AdminError::ConfigError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "CONFIG_ERROR"),
        };

        let body = serde_json::json!({
            "error": self.to_string(),
            "code": code,
        });

        (status, Json(body)).into_response()
    }
}

impl From<LicenseError> for AdminError {
    fn from(err: LicenseError) -> Self {
        match err {
            LicenseError::ConfigError(msg) => AdminError::ConfigError(msg),
            LicenseError::ServerError(msg) => AdminError::DatabaseError(msg),
            _ => AdminError::DatabaseError(err.to_string()),
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse an ISO 8601 datetime string into NaiveDateTime.
fn parse_datetime(s: &str) -> Result<NaiveDateTime, AdminError> {
    // Try parsing with time
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Ok(dt.naive_utc());
    }

    // Try parsing date only (assume end of day)
    if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(23, 59, 59).unwrap());
    }

    // Try common formats
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(dt);
    }

    Err(AdminError::BadRequest(format!(
        "invalid datetime format: {s}. Use ISO 8601 (e.g., '2025-12-31T23:59:59Z' or '2025-12-31')"
    )))
}

/// Merge tier features with explicit features.
fn resolve_features(tier: Option<&str>, explicit_features: &[String]) -> Vec<String> {
    let mut features: Vec<String> = if let Some(tier_name) = tier {
        get_tier_features(tier_name)
    } else {
        Vec::new()
    };

    // Add explicit features (avoiding duplicates)
    for feature in explicit_features {
        if !features.contains(feature) {
            features.push(feature.clone());
        }
    }

    features
}

/// Generate a unique license key, checking for collisions.
async fn generate_unique_license_key(db: &Database) -> LicenseResult<String> {
    let config = get_config()?;
    let key_config: LicenseKeyConfig = (&config.license).into();

    // Try up to 10 times to generate a unique key
    for _ in 0..10 {
        let key = generate_license_key(&key_config);
        if !db.license_key_exists(&key).await? {
            return Ok(key);
        }
    }

    Err(LicenseError::ServerError(
        "failed to generate unique license key after 10 attempts".to_string(),
    ))
}

// ============================================================================
// Handlers
// ============================================================================

/// Create a new license.
///
/// `POST /api/v1/licenses`
pub async fn create_license_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateLicenseRequest>,
) -> Result<(StatusCode, Json<LicenseResponse>), AdminError> {
    info!("Creating new license for org_id={:?}", payload.org_id);

    let now = Utc::now().naive_utc();
    let license_id = Uuid::new_v4().to_string();
    let license_key = generate_unique_license_key(&state.db).await?;

    // Parse expiration date if provided
    let expires_at = payload
        .expires_at
        .as_ref()
        .map(|s| parse_datetime(s))
        .transpose()?;

    // Resolve features from tier and explicit list
    let features = resolve_features(payload.tier.as_deref(), &payload.features);
    let features_json = serde_json::to_string(&features).ok();

    // Serialize metadata
    let metadata_json = payload
        .metadata
        .as_ref()
        .and_then(|m| serde_json::to_string(m).ok());

    let license = License {
        license_id: license_id.clone(),
        client_id: None,
        status: "active".to_string(),
        features: features_json,
        issued_at: now,
        expires_at,
        hardware_id: None,
        signature: None,
        last_heartbeat: None,
        org_id: payload.org_id,
        org_name: payload.org_name,
        license_key: Some(license_key.clone()),
        tier: payload.tier,
        device_name: None,
        device_info: None,
        bound_at: None,
        last_seen_at: None,
        suspended_at: None,
        revoked_at: None,
        revoke_reason: None,
        grace_period_ends_at: None,
        suspension_message: None,
        is_blacklisted: None,
        blacklisted_at: None,
        blacklist_reason: None,
        metadata: metadata_json,
    };

    state.db.insert_license(license.clone()).await?;

    info!(
        "Created license license_id={} license_key={}",
        license_id, license_key
    );

    Ok((StatusCode::CREATED, Json(license.into())))
}

/// Batch create multiple licenses.
///
/// `POST /api/v1/licenses/batch`
pub async fn batch_create_license_handler(
    State(state): State<AppState>,
    Json(payload): Json<BatchCreateLicenseRequest>,
) -> Result<(StatusCode, Json<BatchCreateResponse>), AdminError> {
    if payload.count == 0 {
        return Err(AdminError::BadRequest(
            "count must be greater than 0".to_string(),
        ));
    }

    if payload.count > 1000 {
        return Err(AdminError::BadRequest(
            "count must not exceed 1000".to_string(),
        ));
    }

    info!(
        "Batch creating {} licenses for org_id={:?}",
        payload.count, payload.org_id
    );

    let now = Utc::now().naive_utc();

    // Parse expiration date if provided
    let expires_at = payload
        .expires_at
        .as_ref()
        .map(|s| parse_datetime(s))
        .transpose()?;

    // Resolve features
    let features = resolve_features(payload.tier.as_deref(), &payload.features);
    let features_json = serde_json::to_string(&features).ok();

    let mut licenses = Vec::with_capacity(payload.count as usize);

    for _ in 0..payload.count {
        let license_id = Uuid::new_v4().to_string();
        let license_key = generate_unique_license_key(&state.db).await?;

        let license = License {
            license_id: license_id.clone(),
            client_id: None,
            status: "active".to_string(),
            features: features_json.clone(),
            issued_at: now,
            expires_at,
            hardware_id: None,
            signature: None,
            last_heartbeat: None,
            org_id: payload.org_id.clone(),
            org_name: payload.org_name.clone(),
            license_key: Some(license_key.clone()),
            tier: payload.tier.clone(),
            device_name: None,
            device_info: None,
            bound_at: None,
            last_seen_at: None,
            suspended_at: None,
            revoked_at: None,
            revoke_reason: None,
            grace_period_ends_at: None,
            suspension_message: None,
            is_blacklisted: None,
            blacklisted_at: None,
            blacklist_reason: None,
            metadata: None,
        };

        state.db.insert_license(license).await?;

        licenses.push(LicenseSummary {
            license_id,
            license_key,
        });
    }

    info!("Batch created {} licenses", licenses.len());

    Ok((
        StatusCode::CREATED,
        Json(BatchCreateResponse {
            created: licenses.len() as u32,
            licenses,
        }),
    ))
}

/// Get a license by ID.
///
/// `GET /api/v1/licenses/{license_id}`
pub async fn get_license_handler(
    State(state): State<AppState>,
    Path(license_id): Path<String>,
) -> Result<Json<LicenseResponse>, AdminError> {
    info!("Getting license license_id={}", license_id);

    let license = state
        .db
        .get_license(&license_id)
        .await?
        .ok_or_else(|| AdminError::NotFound(format!("license not found: {license_id}")))?;

    Ok(Json(license.into()))
}

/// List licenses with optional filtering.
///
/// `GET /api/v1/licenses?org_id={id}&page={n}&per_page={n}`
pub async fn list_licenses_handler(
    State(state): State<AppState>,
    Query(query): Query<ListLicensesQuery>,
) -> Result<Json<ListLicensesResponse>, AdminError> {
    info!(
        "Listing licenses org_id={:?} page={} per_page={}",
        query.org_id, query.page, query.per_page
    );

    let licenses = if let Some(org_id) = &query.org_id {
        state.db.list_licenses_by_org(org_id).await?
    } else {
        // For now, require org_id filter to prevent unbounded queries
        return Err(AdminError::BadRequest(
            "org_id query parameter is required".to_string(),
        ));
    };

    let total = licenses.len() as u32;
    let total_pages = total.div_ceil(query.per_page);

    // Apply pagination
    let start = ((query.page.saturating_sub(1)) * query.per_page) as usize;
    let end = (start + query.per_page as usize).min(licenses.len());

    let page_licenses: Vec<LicenseResponse> = licenses
        .into_iter()
        .skip(start)
        .take(end - start)
        .map(|l| l.into())
        .collect();

    Ok(Json(ListLicensesResponse {
        licenses: page_licenses,
        total,
        page: query.page,
        per_page: query.per_page,
        total_pages,
    }))
}

/// Update a license.
///
/// `PATCH /api/v1/licenses/{license_id}`
pub async fn update_license_handler(
    State(state): State<AppState>,
    Path(license_id): Path<String>,
    Json(payload): Json<UpdateLicenseRequest>,
) -> Result<Json<LicenseResponse>, AdminError> {
    info!("Updating license license_id={}", license_id);

    let mut license = state
        .db
        .get_license(&license_id)
        .await?
        .ok_or_else(|| AdminError::NotFound(format!("license not found: {license_id}")))?;

    // Update tier if provided
    if let Some(tier) = &payload.tier {
        license.tier = Some(tier.clone());

        // Re-derive features from tier if features not explicitly provided
        if payload.features.is_none() {
            let features = resolve_features(Some(tier), &[]);
            license.features = serde_json::to_string(&features).ok();
        }
    }

    // Update features if explicitly provided
    if let Some(features) = &payload.features {
        // If tier is also being set, merge with tier features
        let final_features = if let Some(tier) = &payload.tier {
            resolve_features(Some(tier), features)
        } else {
            features.clone()
        };
        license.features = serde_json::to_string(&final_features).ok();
    }

    // Update expiration date if provided
    if let Some(expires_at_str) = &payload.expires_at {
        license.expires_at = Some(parse_datetime(expires_at_str)?);
    }

    // Update metadata if provided
    if let Some(metadata) = &payload.metadata {
        license.metadata = serde_json::to_string(metadata).ok();
    }

    state.db.insert_license(license.clone()).await?;

    info!("Updated license license_id={}", license_id);

    Ok(Json(license.into()))
}

/// Request for admin force release.
#[derive(Debug, Deserialize)]
pub struct AdminReleaseRequest {
    /// Reason for force release (optional, for audit)
    pub reason: Option<String>,
}

/// Response from admin release.
#[derive(Debug, Serialize)]
pub struct AdminReleaseResponse {
    pub success: bool,
    pub message: String,
    pub previous_hardware_id: Option<String>,
    pub previous_device_name: Option<String>,
}

/// Admin force release a license from hardware.
///
/// `POST /api/v1/licenses/{license_id}/release`
///
/// This endpoint allows administrators to force-release a license from its
/// bound hardware, useful when a user loses access to their device.
pub async fn admin_release_handler(
    State(state): State<AppState>,
    Path(license_id): Path<String>,
    Json(payload): Json<AdminReleaseRequest>,
) -> Result<Json<AdminReleaseResponse>, AdminError> {
    use crate::server::database::{BindingAction, PerformedBy};

    info!("Admin release request for license_id={}", license_id);

    // Get the license
    let license = state
        .db
        .get_license(&license_id)
        .await?
        .ok_or_else(|| AdminError::NotFound(format!("License {license_id} not found")))?;

    // Check if bound
    if !license.is_bound() {
        return Err(AdminError::BadRequest(
            "License is not currently bound".to_string(),
        ));
    }

    // Save previous binding info for response
    let previous_hardware_id = license.hardware_id.clone();
    let previous_device_name = license.device_name.clone();

    // Release the license
    state.db.release_license(&license_id).await?;

    // Record in binding history
    let _ = state
        .db
        .record_binding_history(
            &license_id,
            BindingAction::AdminRelease,
            previous_hardware_id.as_deref(),
            previous_device_name.as_deref(),
            license.device_info.as_deref(),
            PerformedBy::Admin,
            payload.reason.as_deref(),
        )
        .await;

    info!(
        "Admin released license {} from hardware {:?}",
        license_id, previous_hardware_id
    );

    Ok(Json(AdminReleaseResponse {
        success: true,
        message: "License released successfully".to_string(),
        previous_hardware_id,
        previous_device_name,
    }))
}

/// Request for revoking a license.
#[derive(Debug, Deserialize)]
pub struct RevokeLicenseRequest {
    /// Reason for revocation (stored in revoke_reason)
    pub reason: Option<String>,
    /// Number of days for grace period (0 = immediate revocation)
    #[serde(default)]
    pub grace_period_days: u32,
    /// Message to display to user during grace period
    pub message: Option<String>,
}

/// Response from revoke operation.
#[derive(Debug, Serialize)]
pub struct RevokeLicenseResponse {
    pub success: bool,
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_ends_at: Option<String>,
}

/// Request for reinstating a license.
#[derive(Debug, Deserialize)]
pub struct ReinstateLicenseRequest {
    /// New expiration date (optional, ISO 8601 format)
    pub new_expires_at: Option<String>,
    /// Whether to reset bandwidth counters (if applicable)
    #[serde(default)]
    pub reset_bandwidth: bool,
    /// Reason for reinstatement (for audit)
    pub reason: Option<String>,
}

/// Response from reinstate operation.
#[derive(Debug, Serialize)]
pub struct ReinstateLicenseResponse {
    pub success: bool,
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Request for extending a license.
#[derive(Debug, Deserialize)]
pub struct ExtendLicenseRequest {
    /// New expiration date (required, ISO 8601 format)
    pub new_expires_at: String,
    /// Whether to reset bandwidth counters (if applicable)
    #[serde(default)]
    pub reset_bandwidth: bool,
    /// Reason for extension (for audit)
    pub reason: Option<String>,
}

/// Response from extend operation.
#[derive(Debug, Serialize)]
pub struct ExtendLicenseResponse {
    pub success: bool,
    pub message: String,
    pub previous_expires_at: Option<String>,
    pub new_expires_at: String,
}

/// Request for updating license usage/quota.
#[derive(Debug, Deserialize)]
pub struct UpdateUsageRequest {
    /// Current bandwidth used in bytes
    pub bandwidth_used_bytes: Option<u64>,
    /// Bandwidth limit in bytes (None = unlimited)
    pub bandwidth_limit_bytes: Option<u64>,
    /// Whether to reset the usage counter to zero
    #[serde(default)]
    pub reset: bool,
}

/// Response from usage update operation.
#[derive(Debug, Serialize)]
pub struct UpdateUsageResponse {
    pub success: bool,
    pub bandwidth_used_bytes: u64,
    pub bandwidth_limit_bytes: Option<u64>,
    pub quota_exceeded: bool,
    pub usage_percentage: Option<f64>,
}

/// Revoke a license.
///
/// `POST /api/v1/licenses/{license_id}/revoke`
///
/// This endpoint revokes a license, optionally with a grace period.
///
/// # Behavior
/// - If `grace_period_days = 0`: Sets status to 'revoked' immediately
/// - If `grace_period_days > 0`: Sets status to 'suspended' with calculated grace_period_ends_at
/// - Stores the revoke_reason and suspension_message for audit/display
pub async fn revoke_license_handler(
    State(state): State<AppState>,
    Path(license_id): Path<String>,
    Json(payload): Json<RevokeLicenseRequest>,
) -> Result<Json<RevokeLicenseResponse>, AdminError> {
    info!(
        "Revoke request for license_id={} grace_period_days={}",
        license_id, payload.grace_period_days
    );

    // Get the license
    let mut license = state
        .db
        .get_license(&license_id)
        .await?
        .ok_or_else(|| AdminError::NotFound(format!("License {license_id} not found")))?;

    // Check if already revoked
    if license.status == "revoked" {
        return Err(AdminError::BadRequest(
            "License is already revoked".to_string(),
        ));
    }

    let now = Utc::now().naive_utc();

    if payload.grace_period_days == 0 {
        // Immediate revocation
        license.status = "revoked".to_string();
        license.revoked_at = Some(now);
        license.revoke_reason = payload.reason.clone();
        license.suspended_at = None;
        license.grace_period_ends_at = None;
        license.suspension_message = None;

        state.db.insert_license(license).await?;

        info!("License {} revoked immediately", license_id);

        Ok(Json(RevokeLicenseResponse {
            success: true,
            status: "revoked".to_string(),
            message: "License has been revoked".to_string(),
            grace_period_ends_at: None,
        }))
    } else {
        // Suspension with grace period
        let grace_end = now + chrono::Duration::days(payload.grace_period_days as i64);

        license.status = "suspended".to_string();
        license.suspended_at = Some(now);
        license.grace_period_ends_at = Some(grace_end);
        license.revoke_reason = payload.reason.clone();
        license.suspension_message = payload.message.clone();
        // Don't set revoked_at yet - that happens when grace period expires

        state.db.insert_license(license).await?;

        info!(
            "License {} suspended with grace period until {}",
            license_id, grace_end
        );

        Ok(Json(RevokeLicenseResponse {
            success: true,
            status: "suspended".to_string(),
            message: format!(
                "License has been suspended with {} day grace period",
                payload.grace_period_days
            ),
            grace_period_ends_at: Some(grace_end.to_string()),
        }))
    }
}

/// Reinstate a revoked or suspended license.
///
/// `POST /api/v1/licenses/{license_id}/reinstate`
///
/// This endpoint reinstates a license that was previously revoked or suspended.
///
/// # Behavior
/// - Sets status back to 'active'
/// - Clears all suspension/revocation fields
/// - Optionally sets a new expiration date
/// - Optionally resets bandwidth counters (if tracked)
pub async fn reinstate_license_handler(
    State(state): State<AppState>,
    Path(license_id): Path<String>,
    Json(payload): Json<ReinstateLicenseRequest>,
) -> Result<Json<ReinstateLicenseResponse>, AdminError> {
    info!(
        "Reinstate request for license_id={} reset_bandwidth={}",
        license_id, payload.reset_bandwidth
    );

    // Get the license
    let mut license = state
        .db
        .get_license(&license_id)
        .await?
        .ok_or_else(|| AdminError::NotFound(format!("License {license_id} not found")))?;

    // Check if license needs reinstatement
    if license.status == "active" {
        return Err(AdminError::BadRequest(
            "License is already active".to_string(),
        ));
    }

    // Set status back to active
    license.status = "active".to_string();

    // Clear suspension/revocation fields
    license.suspended_at = None;
    license.revoked_at = None;
    license.revoke_reason = None;
    license.grace_period_ends_at = None;
    license.suspension_message = None;

    // Update expiration date if provided
    let expires_at_str = if let Some(new_expires_at) = &payload.new_expires_at {
        let expires_at = parse_datetime(new_expires_at)?;
        license.expires_at = Some(expires_at);
        Some(expires_at.to_string())
    } else {
        license.expires_at.map(|dt| dt.to_string())
    };

    // Note: reset_bandwidth would reset any bandwidth tracking if we had it
    // For now, this is a no-op but the field is accepted for future compatibility
    if payload.reset_bandwidth {
        info!(
            "Bandwidth reset requested for license {} (no-op for now)",
            license_id
        );
    }

    state.db.insert_license(license).await?;

    info!(
        "License {} reinstated, reason={:?}",
        license_id, payload.reason
    );

    Ok(Json(ReinstateLicenseResponse {
        success: true,
        status: "active".to_string(),
        message: "License has been reinstated".to_string(),
        expires_at: expires_at_str,
    }))
}

/// Extend a license's expiration date.
///
/// `POST /api/v1/licenses/{license_id}/extend`
///
/// This endpoint extends a license's expiration date.
///
/// # Behavior
/// - Updates the `expires_at` field to the new date
/// - Optionally resets bandwidth counters (when quota tracking is enabled)
/// - Can be used on active, suspended, or revoked licenses
pub async fn extend_license_handler(
    State(state): State<AppState>,
    Path(license_id): Path<String>,
    Json(payload): Json<ExtendLicenseRequest>,
) -> Result<Json<ExtendLicenseResponse>, AdminError> {
    info!(
        "Extend request for license_id={} new_expires_at={}",
        license_id, payload.new_expires_at
    );

    // Get the license
    let mut license = state
        .db
        .get_license(&license_id)
        .await?
        .ok_or_else(|| AdminError::NotFound(format!("License {license_id} not found")))?;

    // Parse the new expiration date
    let new_expires_at = parse_datetime(&payload.new_expires_at)?;

    // Save previous expiration for response
    let previous_expires_at = license.expires_at.map(|dt| dt.to_string());

    // Update expiration date
    license.expires_at = Some(new_expires_at);

    // Note: reset_bandwidth would reset any bandwidth tracking if we had it
    // For now, this is a no-op but the field is accepted for future compatibility
    if payload.reset_bandwidth {
        info!(
            "Bandwidth reset requested for license {} (no-op for now)",
            license_id
        );
    }

    state.db.insert_license(license).await?;

    info!(
        "License {} extended to {}, reason={:?}",
        license_id, new_expires_at, payload.reason
    );

    Ok(Json(ExtendLicenseResponse {
        success: true,
        message: "License expiration has been extended".to_string(),
        previous_expires_at,
        new_expires_at: new_expires_at.to_string(),
    }))
}

/// Update license usage/bandwidth tracking.
///
/// `PATCH /api/v1/licenses/{license_id}/usage`
///
/// This endpoint updates the bandwidth usage tracking for a license.
///
/// # Behavior
/// - Sets `bandwidth_used_bytes` to the provided value (or resets to 0 if `reset: true`)
/// - Sets `bandwidth_limit_bytes` if provided
/// - Calculates `quota_exceeded` flag based on usage vs limit
/// - Returns usage statistics including percentage used
///
/// # Note
/// Currently, usage data is not persisted to database (requires `quota-tracking` feature).
/// This endpoint provides the API contract for when quota tracking is implemented.
pub async fn update_usage_handler(
    State(state): State<AppState>,
    Path(license_id): Path<String>,
    Json(payload): Json<UpdateUsageRequest>,
) -> Result<Json<UpdateUsageResponse>, AdminError> {
    info!(
        "Update usage request for license_id={} used={:?} limit={:?} reset={}",
        license_id, payload.bandwidth_used_bytes, payload.bandwidth_limit_bytes, payload.reset
    );

    // Verify license exists
    let license = state
        .db
        .get_license(&license_id)
        .await?
        .ok_or_else(|| AdminError::NotFound(format!("License {license_id} not found")))?;

    // Calculate usage values
    let bandwidth_used_bytes = if payload.reset {
        0
    } else {
        payload.bandwidth_used_bytes.unwrap_or(0)
    };

    let bandwidth_limit_bytes = payload.bandwidth_limit_bytes;

    // Calculate quota exceeded
    let quota_exceeded = match bandwidth_limit_bytes {
        Some(limit) if limit > 0 => bandwidth_used_bytes >= limit,
        _ => false,
    };

    // Calculate usage percentage
    let usage_percentage = bandwidth_limit_bytes.map(|limit| {
        if limit > 0 {
            (bandwidth_used_bytes as f64 / limit as f64) * 100.0
        } else {
            0.0
        }
    });

    // NOTE: When quota-tracking feature is implemented, persist these values:
    // license.bandwidth_used_bytes = Some(bandwidth_used_bytes);
    // license.bandwidth_limit_bytes = bandwidth_limit_bytes;
    // license.quota_exceeded = Some(quota_exceeded);
    // state.db.insert_license(license).await?;

    info!(
        "Usage updated for license {} (not persisted until quota-tracking feature): used={} limit={:?} exceeded={}",
        license_id, bandwidth_used_bytes, bandwidth_limit_bytes, quota_exceeded
    );

    // We still need to touch the license to verify it exists
    let _ = license;

    Ok(Json(UpdateUsageResponse {
        success: true,
        bandwidth_used_bytes,
        bandwidth_limit_bytes,
        quota_exceeded,
        usage_percentage,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_datetime_rfc3339() {
        let dt = parse_datetime("2025-12-31T23:59:59Z").unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 31);
    }

    #[test]
    fn parse_datetime_date_only() {
        let dt = parse_datetime("2025-12-31").unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 31);
        assert_eq!(dt.hour(), 23);
        assert_eq!(dt.minute(), 59);
    }

    #[test]
    fn parse_datetime_invalid() {
        assert!(parse_datetime("invalid").is_err());
        assert!(parse_datetime("2025/12/31").is_err());
    }

    #[test]
    fn resolve_features_no_tier() {
        let features = resolve_features(None, &["feature_a".to_string()]);
        assert_eq!(features, vec!["feature_a"]);
    }

    #[test]
    fn resolve_features_merges_without_duplicates() {
        // Without a configured tier, just returns explicit features
        let features = resolve_features(
            Some("nonexistent"),
            &["feature_a".to_string(), "feature_b".to_string()],
        );
        assert_eq!(features, vec!["feature_a", "feature_b"]);
    }

    #[test]
    fn license_response_from_license() {
        let license = License {
            license_id: "test-id".to_string(),
            client_id: None,
            status: "active".to_string(),
            features: Some(r#"["feature_a","feature_b"]"#.to_string()),
            issued_at: Utc::now().naive_utc(),
            expires_at: None,
            hardware_id: Some("hw-123".to_string()),
            signature: None,
            last_heartbeat: None,
            org_id: Some("org-1".to_string()),
            org_name: Some("Test Org".to_string()),
            license_key: Some("LIC-AAAA-BBBB-CCCC".to_string()),
            tier: Some("pro".to_string()),
            device_name: Some("Test Device".to_string()),
            device_info: None,
            bound_at: Some(Utc::now().naive_utc()),
            last_seen_at: None,
            suspended_at: None,
            revoked_at: None,
            revoke_reason: None,
            grace_period_ends_at: None,
            suspension_message: None,
            is_blacklisted: None,
            blacklisted_at: None,
            blacklist_reason: None,
            metadata: Some(r#"{"key":"value"}"#.to_string()),
        };

        let response: LicenseResponse = license.into();

        assert_eq!(response.license_id, "test-id");
        assert_eq!(response.status, "active");
        assert_eq!(response.features, vec!["feature_a", "feature_b"]);
        assert_eq!(response.org_id, Some("org-1".to_string()));
        assert_eq!(response.tier, Some("pro".to_string()));
        assert!(response.is_bound);
        assert!(response.metadata.is_some());
    }

    #[test]
    fn admin_error_display() {
        assert!(AdminError::NotFound("test".to_string())
            .to_string()
            .contains("not found"));
        assert!(AdminError::BadRequest("test".to_string())
            .to_string()
            .contains("bad request"));
    }
}
