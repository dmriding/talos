use std::sync::Arc;
use sqlx::types::chrono::Utc;
use crate::server::database::{Database, License};
use chrono::NaiveDateTime;

pub async fn activate_license_handler(
    db: Arc<Database>,
    license_id: String,
    client_id: String,
    features: Option<String>,
    hardware_id: Option<String>,
    expires_at: Option<NaiveDateTime>
) -> bool {
    let license = License {
        license_id,
        client_id,
        status: "active".to_string(),
        features,
        issued_at: Utc::now().naive_utc(),
        expires_at,
        hardware_id,
        signature: None,
    };

    db.insert_license(license).await.is_ok()
}

pub async fn validate_license_handler(
    db: Arc<Database>,
    license_id: String
) -> bool {
    if let Ok(Some(license)) = db.get_license(&license_id).await {
        return license.status == "active";
    }
    false
}

pub async fn deactivate_license_handler(
    db: Arc<Database>,
    license_id: String
) -> bool {
    if let Ok(Some(mut license)) = db.get_license(&license_id).await {
        license.status = "inactive".to_string();
        db.insert_license(license).await.is_ok()
    } else {
        false
    }
}
