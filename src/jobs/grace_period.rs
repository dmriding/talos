//! Grace period expiration job.
//!
//! This job checks for licenses that are in 'suspended' status with a grace period
//! that has expired, and updates their status to 'revoked'.

use chrono::Utc;
use tracing::{debug, info};

use crate::server::database::Database;

use super::JobError;

/// Check for and process licenses with expired grace periods.
///
/// Queries for licenses where:
/// - `status = 'suspended'`
/// - `grace_period_ends_at < NOW()`
///
/// Updates matching licenses:
/// - Sets `status = 'revoked'`
/// - Sets `revoked_at = NOW()`
///
/// Returns the number of licenses that were revoked.
pub async fn run_grace_period_check(db: &Database) -> Result<u32, JobError> {
    let now = Utc::now().naive_utc();

    debug!("Checking for expired grace periods at {}", now);

    // Get all suspended licenses with expired grace periods
    let expired_licenses = db.get_expired_grace_period_licenses(now).await?;

    let mut count = 0;

    for license in expired_licenses {
        debug!(
            "Revoking license {} (grace period ended at {:?})",
            license.license_id, license.grace_period_ends_at
        );

        // Update the license status to revoked
        let mut updated = license.clone();
        updated.status = "revoked".to_string();
        updated.revoked_at = Some(now);

        if db.insert_license(updated).await.is_ok() {
            count += 1;
            info!(
                "License {} revoked (grace period expired)",
                license.license_id
            );
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    // Integration tests are in tests/jobs_tests.rs
}
