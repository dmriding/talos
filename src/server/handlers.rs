use std::sync::{Arc, Mutex};
use crate::server::database::LicenseDB;

pub async fn activate_license_handler(
    db: &Arc<Mutex<LicenseDB>>,
    license_id: String,
    client_id: String,
) -> bool {
    let mut db = db.lock().unwrap();
    db.insert(format!("{}_{}", license_id, client_id), true);
    true
}

pub async fn validate_license_handler(
    db: &Arc<Mutex<LicenseDB>>,
    license_id: String,
    client_id: String,
) -> bool {
    let db = db.lock().unwrap();
    *db.get(&format!("{}_{}", license_id, client_id)).unwrap_or(&false)
}

pub async fn deactivate_license_handler(
    db: &Arc<Mutex<LicenseDB>>,
    license_id: String,
    client_id: String,
) -> bool {
    let mut db = db.lock().unwrap();
    db.insert(format!("{}_{}", license_id, client_id), false);
    true
}
