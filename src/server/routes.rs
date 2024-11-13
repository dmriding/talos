use axum::{Json, extract::State};
use crate::server::handlers::{activate_license_handler, validate_license_handler, deactivate_license_handler};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use crate::server::database::LicenseDB;

pub type SharedDB = Arc<Mutex<LicenseDB>>;

#[derive(Debug, Deserialize, Serialize)]
pub struct LicenseRequest {
    license_id: String,
    client_id: String,
}

pub async fn activate_license(
    State(db): State<SharedDB>,
    Json(payload): Json<LicenseRequest>,
) -> Json<bool> {
    let result = activate_license_handler(&db, payload.license_id, payload.client_id).await;
    Json(result)
}

pub async fn validate_license(
    State(db): State<SharedDB>,
    Json(payload): Json<LicenseRequest>,
) -> Json<bool> {
    let result = validate_license_handler(&db, payload.license_id, payload.client_id).await;
    Json(result)
}

pub async fn deactivate_license(
    State(db): State<SharedDB>,
    Json(payload): Json<LicenseRequest>,
) -> Json<bool> {
    let result = deactivate_license_handler(&db, payload.license_id, payload.client_id).await;
    Json(result)
}
