//! License expiration job.
//!
//! This job checks for licenses that are in 'active' status with an expiration date
//! that has passed, and updates their status to 'expired'.

use chrono::Utc;
use tracing::{debug, info};

use crate::server::database::Database;

use super::JobError;

/// Check for and process expired licenses.
///
/// Queries for licenses where:
/// - `status = 'active'`
/// - `expires_at < NOW()`
///
/// Updates matching licenses:
/// - Sets `status = 'expired'`
///
/// Returns the number of licenses that were expired.
pub async fn run_license_expiration_check(db: &Database) -> Result<u32, JobError> {
    let now = Utc::now().naive_utc();

    debug!("Checking for expired licenses at {}", now);

    // Get all active licenses with expired dates
    let expired_licenses = db.get_expired_licenses(now).await?;

    let mut count = 0;

    for license in expired_licenses {
        debug!(
            "Expiring license {} (expired at {:?})",
            license.license_id, license.expires_at
        );

        // Update the license status to expired
        let mut updated = license.clone();
        updated.status = "expired".to_string();

        if db.insert_license(updated).await.is_ok() {
            count += 1;
            info!("License {} expired", license.license_id);
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    // Integration tests are in tests/jobs_tests.rs
}
