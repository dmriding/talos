use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref LICENSE_DB: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new());
}

/// Simulate server-side license activation
pub fn activate_license(license_id: &str, client_id: &str) -> bool {
    let mut db = LICENSE_DB.lock().unwrap();
    db.insert(format!("{}_{}", license_id, client_id), true);
    true
}

/// Simulate server-side license deactivation
pub fn deactivate_license(license_id: &str, client_id: &str) -> bool {
    let mut db = LICENSE_DB.lock().unwrap();
    db.insert(format!("{}_{}", license_id, client_id), false);
    true
}
