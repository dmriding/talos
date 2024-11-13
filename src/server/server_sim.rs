use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;
use std::error::Error;

lazy_static! {
    /// A simulated in-memory database to store license states.
    static ref LICENSE_DB: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new());
}

/// Simulate server-side license activation
pub fn activate_license(license_id: &str, client_id: &str) -> Result<bool, Box<dyn Error>> {
    // Lock the database and update it
    let mut db = LICENSE_DB.lock().map_err(|_| "Failed to acquire lock on LICENSE_DB")?;
    db.insert(format!("{}_{}", license_id, client_id), true);
    Ok(true)
}

/// Simulate server-side license deactivation
pub fn deactivate_license(license_id: &str, client_id: &str) -> Result<bool, Box<dyn Error>> {
    // Lock the database and update it
    let mut db = LICENSE_DB.lock().map_err(|_| "Failed to acquire lock on LICENSE_DB")?;
    db.insert(format!("{}_{}", license_id, client_id), false);
    Ok(true)
}

/// Check if a license is active
pub fn is_license_active(license_id: &str, client_id: &str) -> Result<bool, Box<dyn Error>> {
    // Lock the database and check the license status
    let db = LICENSE_DB.lock().map_err(|_| "Failed to acquire lock on LICENSE_DB")?;
    Ok(*db.get(&format!("{}_{}", license_id, client_id)).unwrap_or(&false))
}
