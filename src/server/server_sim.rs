use std::collections::HashMap;
use std::sync::Mutex;

use lazy_static::lazy_static;

use crate::errors::{LicenseError, LicenseResult};

lazy_static! {
    /// A simulated in-memory database to store license states.
    ///
    /// Key format: "{license_id}_{client_id}"
    /// Value: `true`  -> active
    ///        `false` -> inactive
    static ref LICENSE_DB: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new());
}

/// Build a composite key from license_id + client_id.
fn make_key(license_id: &str, client_id: &str) -> String {
    format!("{}_{}", license_id, client_id)
}

/// Simulate server-side license activation.
///
/// - Marks the (license_id, client_id) pair as active in the in-memory DB.
pub fn activate_license(license_id: &str, client_id: &str) -> LicenseResult<bool> {
    let mut db = LICENSE_DB
        .lock()
        .map_err(|_| LicenseError::ServerError("failed to acquire LICENSE_DB lock".into()))?;

    db.insert(make_key(license_id, client_id), true);
    Ok(true)
}

/// Simulate server-side license deactivation.
///
/// - Marks the (license_id, client_id) pair as inactive.
pub fn deactivate_license(license_id: &str, client_id: &str) -> LicenseResult<bool> {
    let mut db = LICENSE_DB
        .lock()
        .map_err(|_| LicenseError::ServerError("failed to acquire LICENSE_DB lock".into()))?;

    db.insert(make_key(license_id, client_id), false);
    Ok(true)
}

/// Check if a license is active in the simulated server.
///
/// - Returns `Ok(true)` if the license exists and is active.
/// - Returns `Ok(false)` if it does not exist or is inactive.
pub fn is_license_active(license_id: &str, client_id: &str) -> LicenseResult<bool> {
    let db = LICENSE_DB
        .lock()
        .map_err(|_| LicenseError::ServerError("failed to acquire LICENSE_DB lock".into()))?;

    Ok(*db.get(&make_key(license_id, client_id)).unwrap_or(&false))
}
