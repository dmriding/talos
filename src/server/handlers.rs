use std::sync::Arc;
use axum::{Json, extract::State};
use crate::server::database::{Database, License};
use serde::{Deserialize, Serialize};
use chrono::Utc;

#[derive(Debug, Deserialize, Serialize)]
pub struct LicenseRequest {
    pub license_id: String,
    pub client_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LicenseResponse {
    pub success: bool,
}

/// Handler for activating a license
pub async fn activate_license_handler(
    State(db): State<Arc<Database>>,
    Json(payload): Json<LicenseRequest>,
) -> Json<LicenseResponse> {
    let license = License {
        license_id: payload.license_id.clone(),
        client_id: payload.client_id,
        status: "active".to_string(),
        features: None,
        issued_at: Utc::now().naive_utc(),
        expires_at: None,
        hardware_id: None,
        signature: None,
    };

    let success = db.insert_license(license).await.is_ok();
    Json(LicenseResponse { success })
}

/// Handler for validating a license
pub async fn validate_license_handler(
    State(db): State<Arc<Database>>,
    Json(payload): Json<LicenseRequest>,
) -> Json<LicenseResponse> {
    let success = match db.get_license(&payload.license_id).await {
        Ok(Some(license)) => license.status == "active",
        _ => false,
    };
    Json(LicenseResponse { success })
}

/// Handler for deactivating a license
pub async fn deactivate_license_handler(
    State(db): State<Arc<Database>>,
    Json(payload): Json<LicenseRequest>,
) -> Json<LicenseResponse> {
    println!("Received request to deactivate license: {:?}", payload);

    // Try to fetch the license from the database
    let success = match db.get_license(&payload.license_id).await {
        Ok(Some(mut license)) => {
            // Check if the client_id matches before deactivating
            if license.client_id == payload.client_id {
                println!("License found and client ID matches, deactivating...");
                
                // Update the license status to "inactive"
                license.status = "inactive".to_string();
                
                // Save the updated license back to the database
                let update_result = db.insert_license(license).await.is_ok();
                println!("License deactivation result: {}", update_result);
                update_result
            } else {
                println!("Client ID does not match.");
                false
            }
        }
        Ok(None) => {
            println!("License not found for ID: {}", payload.license_id);
            false
        }
        Err(err) => {
            println!("Error fetching license: {:?}", err);
            false
        }
    };

    Json(LicenseResponse { success })
}
