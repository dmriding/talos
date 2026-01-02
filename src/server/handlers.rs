use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::errors::{LicenseError, LicenseResult};
use crate::server::database::{Database, License};

/// Shared application state for handlers.
///
/// Right now this only wraps the database, but later you can add:
/// config, key material, metrics handles, etc.
/// without touching every handler signature.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
}

/// Standard error response body for HTTP errors.
#[derive(Debug, Serialize)]
struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

/// Map internal LicenseError into an HTTP response Axum understands.
///
/// This lets handlers return:
///   Result<Json<T>, LicenseError>
/// and Axum will convert both success and error into HTTP responses.
impl IntoResponse for LicenseError {
    fn into_response(self) -> Response {
        // Map error categories to status codes.
        let status = match self {
            LicenseError::InvalidLicense(_) => StatusCode::BAD_REQUEST,
            LicenseError::ConfigError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            LicenseError::NetworkError(_) => StatusCode::BAD_GATEWAY,
            LicenseError::StorageError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            LicenseError::EncryptionError(_) | LicenseError::DecryptionError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            LicenseError::ServerError(_) | LicenseError::UnknownError => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let body = ErrorResponse {
            success: false,
            error: self.to_string(),
        };

        (status, Json(body)).into_response()
    }
}

/// Request structure for license-related operations.
#[derive(Debug, Deserialize, Serialize)]
pub struct LicenseRequest {
    pub license_id: String,
    pub client_id: String,
}

/// Response structure for license-related operations.
#[derive(Debug, Deserialize, Serialize)]
pub struct LicenseResponse {
    pub success: bool,
}

/// Request structure for heartbeat operations.
///
/// Kept separate in case heartbeat later includes extra metadata.
#[derive(Debug, Deserialize, Serialize)]
pub struct HeartbeatRequest {
    pub license_id: String,
    pub client_id: String,
}

/// Response structure for heartbeat operations.
#[derive(Debug, Deserialize, Serialize)]
pub struct HeartbeatResponse {
    pub success: bool,
}

/// Handler for activating a license.
///
/// Behavior:
/// - If the license does not exist, it is created as `active`.
/// - If it exists, it is updated to `active` with the given client_id.
/// - DB errors bubble up as `LicenseError` (mapped to HTTP 5xx).
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
