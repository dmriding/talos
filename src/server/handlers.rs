use std::sync::Arc;

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::errors::{LicenseError, LicenseResult};
use crate::server::api_error::ApiError;
use crate::server::database::{Database, License};

#[cfg(feature = "jwt-auth")]
use crate::server::auth::AuthState;

/// Shared application state for handlers.
///
/// Right now this only wraps the database, but later you can add:
/// config, key material, metrics handles, etc.
/// without touching every handler signature.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    #[cfg(feature = "jwt-auth")]
    pub auth: AuthState,
}

/// Map internal LicenseError into an HTTP response Axum understands.
///
/// This lets handlers return:
///   Result<Json<T>, LicenseError>
/// and Axum will convert both success and error into HTTP responses.
///
/// Uses the standardized `ApiError` format for consistent error responses.
impl IntoResponse for LicenseError {
    fn into_response(self) -> Response {
        let api_error: ApiError = self.into();
        api_error.into_response()
    }
}

/// Request structure for license-related operations.
#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct LicenseRequest {
    pub license_id: String,
    pub client_id: String,
}

/// Response structure for license-related operations.
#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct LicenseResponse {
    pub success: bool,
}

/// Request structure for heartbeat operations.
///
/// Kept separate in case heartbeat later includes extra metadata.
#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct HeartbeatRequest {
    pub license_id: String,
    pub client_id: String,
}

/// Response structure for heartbeat operations.
#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct HeartbeatResponse {
    pub success: bool,
}

/// Handler for activating a license.
///
/// Behavior:
/// - If the license does not exist, it is created as `active`.
/// - If it exists, it is updated to `active` with the given client_id.
/// - DB errors bubble up as `LicenseError` (mapped to HTTP 5xx).
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/activate",
    tag = "legacy",
    request_body = LicenseRequest,
    responses(
        (status = 200, description = "License activated", body = LicenseResponse),
        (status = 500, description = "Server error"),
    )
))]
pub async fn activate_license_handler(
    State(state): State<AppState>,
    Json(payload): Json<LicenseRequest>,
) -> LicenseResult<Json<LicenseResponse>> {
    info!(
        "Activating license_id={} for client_id={}",
        payload.license_id, payload.client_id
    );

    let now = Utc::now().naive_utc();

    let license = License {
        license_id: payload.license_id.clone(),
        client_id: Some(payload.client_id.clone()),
        status: "active".to_string(),
        features: None,
        issued_at: now,
        expires_at: None,
        hardware_id: None,
        signature: None,
        last_heartbeat: Some(now),
        // Extended fields (all optional, default to None)
        org_id: None,
        org_name: None,
        license_key: None,
        tier: None,
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
        bandwidth_used_bytes: None,
        bandwidth_limit_bytes: None,
        quota_exceeded: None,
    };

    state.db.insert_license(license).await?;

    Ok(Json(LicenseResponse { success: true }))
}

/// Handler for validating a license.
///
/// Returns `success: true` only if:
/// - license exists
/// - client_id matches
/// - status == "active"
///
/// DB failures bubble as `LicenseError` (HTTP 5xx).
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/validate",
    tag = "legacy",
    request_body = LicenseRequest,
    responses(
        (status = 200, description = "Validation result", body = LicenseResponse),
        (status = 500, description = "Server error"),
    )
))]
pub async fn validate_license_handler(
    State(state): State<AppState>,
    Json(payload): Json<LicenseRequest>,
) -> LicenseResult<Json<LicenseResponse>> {
    info!(
        "Validating license_id={} for client_id={}",
        payload.license_id, payload.client_id
    );

    let license_opt = state.db.get_license(&payload.license_id).await?;

    let success = match license_opt {
        Some(license) => {
            if license.client_id.as_deref() != Some(payload.client_id.as_str()) {
                warn!(
                    "Client ID mismatch for license_id={} (expected={:?}, got={})",
                    payload.license_id, license.client_id, payload.client_id
                );
                false
            } else if license.status != "active" {
                warn!(
                    "License is not active for license_id={} (status={})",
                    payload.license_id, license.status
                );
                false
            } else {
                true
            }
        }
        None => {
            warn!("License not found for license_id={}", payload.license_id);
            false
        }
    };

    Ok(Json(LicenseResponse { success }))
}

/// Handler for deactivating a license.
///
/// Returns `success: true` only if:
/// - license exists
/// - client_id matches
/// - status successfully updated to "inactive"
///
/// DB failures bubble as `LicenseError`.
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/deactivate",
    tag = "legacy",
    request_body = LicenseRequest,
    responses(
        (status = 200, description = "Deactivation result", body = LicenseResponse),
        (status = 500, description = "Server error"),
    )
))]
pub async fn deactivate_license_handler(
    State(state): State<AppState>,
    Json(payload): Json<LicenseRequest>,
) -> LicenseResult<Json<LicenseResponse>> {
    info!(
        "Deactivating license_id={} for client_id={}",
        payload.license_id, payload.client_id
    );

    let license_opt = state.db.get_license(&payload.license_id).await?;

    let success = if let Some(mut license) = license_opt {
        if license.client_id.as_deref() == Some(payload.client_id.as_str()) {
            license.status = "inactive".to_string();
            state.db.insert_license(license).await?;
            info!(
                "License deactivated for license_id={} client_id={}",
                payload.license_id, payload.client_id
            );
            true
        } else {
            warn!(
                "Client ID mismatch during deactivation for license_id={} (expected={:?}, got={})",
                payload.license_id, license.client_id, payload.client_id
            );
            false
        }
    } else {
        warn!(
            "Deactivation requested for non-existent license_id={}",
            payload.license_id
        );
        false
    };

    Ok(Json(LicenseResponse { success }))
}

/// Handler for the heartbeat mechanism.
///
/// Updates `last_heartbeat` if a matching license + client exists.
/// Returns:
/// - `success: true` if at least one row was updated
/// - `success: false` otherwise
///
/// DB failures bubble as `LicenseError`.
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/heartbeat",
    tag = "legacy",
    request_body = HeartbeatRequest,
    responses(
        (status = 200, description = "Heartbeat result", body = HeartbeatResponse),
        (status = 500, description = "Server error"),
    )
))]
pub async fn heartbeat_handler(
    State(state): State<AppState>,
    Json(payload): Json<HeartbeatRequest>,
) -> LicenseResult<Json<HeartbeatResponse>> {
    info!(
        "Received heartbeat for license_id={} client_id={}",
        payload.license_id, payload.client_id
    );

    let updated = state
        .db
        .update_last_heartbeat(&payload.license_id, &payload.client_id)
        .await?;

    if !updated {
        warn!(
            "Failed to update heartbeat: no matching license for license_id={} client_id={}",
            payload.license_id, payload.client_id
        );
    }

    Ok(Json(HeartbeatResponse { success: updated }))
}
