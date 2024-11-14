use axum::{Json, extract::State};
use crate::server::handlers::{activate_license_handler, validate_license_handler, deactivate_license_handler};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::server::database::Database;
use chrono::NaiveDateTime;

#[derive(Debug, Deserialize, Serialize)]
pub struct LicenseRequest {
    license_id: String,
    client_id: String,
    features: Option<String>,
    hardware_id: Option<String>,
    expires_at: Option<NaiveDateTime>,
}

pub async fn activate_license(
    State(db): State<Arc<Database>>,
    Json(payload): Json<LicenseRequest>
) -> Json<bool> {
    let result = activate_license_handler(
        db,
        payload.license_id,
        payload.client_id,
        payload.features,
        payload.hardware_id,
        payload.expires_at
    ).await;
    Json(result)
}
