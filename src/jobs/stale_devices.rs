//! Stale device cleanup job.
//!
//! This job checks for licenses that are bound to hardware but haven't been seen
//! for a configurable period, and releases them automatically.

use chrono::{Duration, Utc};
use tracing::{debug, info};

use crate::server::database::{BindingAction, Database, PerformedBy};

use super::JobError;

/// Check for and release licenses from stale devices.
///
/// Queries for licenses where:
/// - `hardware_id IS NOT NULL` (bound)
/// - `last_seen_at < NOW() - stale_days`
///
/// Releases matching licenses:
/// - Clears hardware binding fields
/// - Records in binding history with `performed_by: "system"`
///
/// Returns the number of licenses that were released.
pub async fn run_stale_device_cleanup(db: &Database, stale_days: u32) -> Result<u32, JobError> {
    let now = Utc::now().naive_utc();
    let threshold = now - Duration::days(stale_days as i64);

    debug!(
        "Checking for stale devices (last seen before {}) at {}",
        threshold, now
    );

    // Get all licenses with stale devices
    let stale_licenses = db.get_stale_device_licenses(threshold).await?;

    let mut count = 0;

    for license in stale_licenses {
        debug!(
            "Releasing license {} from stale device {} (last seen {:?})",
            license.license_id,
            license.hardware_id.as_deref().unwrap_or("unknown"),
            license.last_seen_at
        );

        // Save binding info for audit
        let hardware_id = license.hardware_id.clone();
        let device_name = license.device_name.clone();
        let device_info = license.device_info.clone();

        // Release the license
        if db.release_license(&license.license_id).await.is_ok() {
            // Record in binding history
            let _ = db
                .record_binding_history(
                    &license.license_id,
                    BindingAction::SystemRelease,
                    hardware_id.as_deref(),
                    device_name.as_deref(),
                    device_info.as_deref(),
                    PerformedBy::System,
                    Some(&format!(
                        "Automatic release: device not seen for {} days",
                        stale_days
                    )),
                )
                .await;

            count += 1;
            info!(
                "License {} released from stale device {}",
                license.license_id,
                hardware_id.unwrap_or_default()
            );
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    // Integration tests are in tests/jobs_tests.rs
}
