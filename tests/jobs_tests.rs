//! Integration tests for background jobs.
//!
//! These tests require the `background-jobs` feature to be enabled.

#![cfg(feature = "background-jobs")]

use chrono::{Duration, Utc};
use std::sync::Arc;
use talos::jobs::{
    run_grace_period_check, run_license_expiration_check, run_stale_device_cleanup, JobConfig,
};
use talos::server::database::Database;

/// Helper to create a test database.
async fn setup_test_db() -> Arc<Database> {
    std::env::set_var("TALOS_DATABASE_TYPE", "sqlite");
    std::env::set_var("TALOS_DATABASE_URL", "sqlite::memory:");

    let db = Database::new().await.expect("failed to create database");

    // Run migrations (create tables)
    match &*db {
        #[cfg(feature = "sqlite")]
        Database::SQLite(pool) => {
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS licenses (
                    license_id TEXT PRIMARY KEY,
                    client_id TEXT,
                    status TEXT NOT NULL DEFAULT 'active',
                    features TEXT,
                    issued_at TEXT NOT NULL,
                    expires_at TEXT,
                    hardware_id TEXT,
                    signature TEXT,
                    last_heartbeat TEXT,
                    org_id TEXT,
                    org_name TEXT,
                    license_key TEXT UNIQUE,
                    tier TEXT,
                    device_name TEXT,
                    device_info TEXT,
                    bound_at TEXT,
                    last_seen_at TEXT,
                    suspended_at TEXT,
                    revoked_at TEXT,
                    revoke_reason TEXT,
                    grace_period_ends_at TEXT,
                    suspension_message TEXT,
                    is_blacklisted INTEGER,
                    blacklisted_at TEXT,
                    blacklist_reason TEXT,
                    metadata TEXT
                )
                "#,
            )
            .execute(pool)
            .await
            .expect("failed to create licenses table");

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS license_binding_history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    license_id TEXT NOT NULL,
                    action TEXT NOT NULL,
                    hardware_id TEXT,
                    device_name TEXT,
                    device_info TEXT,
                    performed_by TEXT NOT NULL,
                    reason TEXT,
                    timestamp TEXT NOT NULL,
                    FOREIGN KEY (license_id) REFERENCES licenses(license_id)
                )
                "#,
            )
            .execute(pool)
            .await
            .expect("failed to create license_binding_history table");
        }
        #[cfg(feature = "postgres")]
        Database::Postgres(_) => {
            panic!("PostgreSQL not supported in tests");
        }
    }

    db
}

/// Helper to create a license with specific status and timestamps.
async fn create_test_license(
    db: &Database,
    license_id: &str,
    status: &str,
    expires_at: Option<chrono::NaiveDateTime>,
    grace_period_ends_at: Option<chrono::NaiveDateTime>,
    hardware_id: Option<&str>,
    last_seen_at: Option<chrono::NaiveDateTime>,
) {
    use talos::server::database::License;

    let now = Utc::now().naive_utc();

    let license = License {
        license_id: license_id.to_string(),
        client_id: None,
        status: status.to_string(),
        features: None,
        issued_at: now,
        expires_at,
        hardware_id: hardware_id.map(|s| s.to_string()),
        signature: None,
        last_heartbeat: None,
        org_id: Some("test-org".to_string()),
        org_name: None,
        license_key: Some(format!("LIC-{}", license_id)),
        tier: None,
        device_name: hardware_id.map(|_| "Test Device".to_string()),
        device_info: None,
        bound_at: hardware_id.map(|_| now),
        last_seen_at,
        suspended_at: if status == "suspended" {
            Some(now)
        } else {
            None
        },
        revoked_at: if status == "revoked" { Some(now) } else { None },
        revoke_reason: None,
        grace_period_ends_at,
        suspension_message: None,
        is_blacklisted: None,
        blacklisted_at: None,
        blacklist_reason: None,
        metadata: None,
    };

    db.insert_license(license)
        .await
        .expect("failed to insert license");
}

// ============================================================================
// Grace Period Expiration Tests
// ============================================================================

#[tokio::test]
async fn grace_period_check_revokes_expired_suspended_licenses() {
    let db = setup_test_db().await;

    let now = Utc::now().naive_utc();
    let past = now - Duration::days(1);

    // Create a suspended license with expired grace period
    create_test_license(
        &*db,
        "grace-expired-1",
        "suspended",
        None,
        Some(past), // Grace period ended yesterday
        None,
        None,
    )
    .await;

    // Create a suspended license with future grace period
    create_test_license(
        &*db,
        "grace-future-1",
        "suspended",
        None,
        Some(now + Duration::days(7)), // Grace period ends in 7 days
        None,
        None,
    )
    .await;

    // Create an active license (should not be affected)
    create_test_license(&*db, "active-1", "active", None, None, None, None).await;

    // Run the grace period check
    let count = run_grace_period_check(&*db).await.expect("job failed");

    // Only the expired suspended license should be revoked
    assert_eq!(count, 1);

    // Verify the license was revoked
    let license = db.get_license("grace-expired-1").await.unwrap().unwrap();
    assert_eq!(license.status, "revoked");
    assert!(license.revoked_at.is_some());

    // Verify the other suspended license is still suspended
    let license = db.get_license("grace-future-1").await.unwrap().unwrap();
    assert_eq!(license.status, "suspended");

    // Verify the active license is still active
    let license = db.get_license("active-1").await.unwrap().unwrap();
    assert_eq!(license.status, "active");
}

#[tokio::test]
async fn grace_period_check_handles_no_expired_licenses() {
    let db = setup_test_db().await;

    let now = Utc::now().naive_utc();

    // Create a suspended license with future grace period
    create_test_license(
        &*db,
        "grace-future-2",
        "suspended",
        None,
        Some(now + Duration::days(7)),
        None,
        None,
    )
    .await;

    // Run the grace period check
    let count = run_grace_period_check(&*db).await.expect("job failed");

    // No licenses should be revoked
    assert_eq!(count, 0);
}

// ============================================================================
// License Expiration Tests
// ============================================================================

#[tokio::test]
async fn license_expiration_check_expires_old_licenses() {
    let db = setup_test_db().await;

    let now = Utc::now().naive_utc();
    let past = now - Duration::days(1);

    // Create an active license that has expired
    create_test_license(
        &*db,
        "expired-1",
        "active",
        Some(past), // Expired yesterday
        None,
        None,
        None,
    )
    .await;

    // Create an active license with future expiration
    create_test_license(
        &*db,
        "future-1",
        "active",
        Some(now + Duration::days(30)), // Expires in 30 days
        None,
        None,
        None,
    )
    .await;

    // Create an active license with no expiration
    create_test_license(&*db, "no-expiry-1", "active", None, None, None, None).await;

    // Run the expiration check
    let count = run_license_expiration_check(&*db)
        .await
        .expect("job failed");

    // Only the expired license should be updated
    assert_eq!(count, 1);

    // Verify the license was expired
    let license = db.get_license("expired-1").await.unwrap().unwrap();
    assert_eq!(license.status, "expired");

    // Verify the other licenses are still active
    let license = db.get_license("future-1").await.unwrap().unwrap();
    assert_eq!(license.status, "active");

    let license = db.get_license("no-expiry-1").await.unwrap().unwrap();
    assert_eq!(license.status, "active");
}

#[tokio::test]
async fn license_expiration_check_ignores_already_expired() {
    let db = setup_test_db().await;

    let now = Utc::now().naive_utc();
    let past = now - Duration::days(1);

    // Create an already expired license (status = 'expired')
    create_test_license(
        &*db,
        "already-expired-1",
        "expired", // Already has expired status
        Some(past),
        None,
        None,
        None,
    )
    .await;

    // Run the expiration check
    let count = run_license_expiration_check(&*db)
        .await
        .expect("job failed");

    // No licenses should be updated (already expired)
    assert_eq!(count, 0);
}

// ============================================================================
// Stale Device Cleanup Tests
// ============================================================================

#[tokio::test]
async fn stale_device_cleanup_releases_old_devices() {
    let db = setup_test_db().await;

    let now = Utc::now().naive_utc();
    let old = now - Duration::days(100);
    let recent = now - Duration::days(10);

    // Create a license with stale device (last seen 100 days ago)
    create_test_license(
        &*db,
        "stale-device-1",
        "active",
        None,
        None,
        Some("hw-stale-1"),
        Some(old),
    )
    .await;

    // Create a license with recent device (last seen 10 days ago)
    create_test_license(
        &*db,
        "recent-device-1",
        "active",
        None,
        None,
        Some("hw-recent-1"),
        Some(recent),
    )
    .await;

    // Create a license with no device
    create_test_license(&*db, "no-device-1", "active", None, None, None, None).await;

    // Run stale device cleanup with 90 day threshold
    let count = run_stale_device_cleanup(&*db, 90)
        .await
        .expect("job failed");

    // Only the stale device should be released
    assert_eq!(count, 1);

    // Verify the stale device was released
    let license = db.get_license("stale-device-1").await.unwrap().unwrap();
    assert!(license.hardware_id.is_none());
    assert!(license.device_name.is_none());

    // Verify the recent device is still bound
    let license = db.get_license("recent-device-1").await.unwrap().unwrap();
    assert!(license.hardware_id.is_some());
}

#[tokio::test]
async fn stale_device_cleanup_handles_no_stale_devices() {
    let db = setup_test_db().await;

    let now = Utc::now().naive_utc();
    let recent = now - Duration::days(10);

    // Create a license with recent device
    create_test_license(
        &*db,
        "recent-device-2",
        "active",
        None,
        None,
        Some("hw-recent-2"),
        Some(recent),
    )
    .await;

    // Run stale device cleanup with 90 day threshold
    let count = run_stale_device_cleanup(&*db, 90)
        .await
        .expect("job failed");

    // No devices should be released
    assert_eq!(count, 0);
}

// ============================================================================
// JobConfig Tests
// ============================================================================

#[test]
fn job_config_has_sensible_defaults() {
    let config = JobConfig::default();

    assert!(!config.stale_device_cleanup_enabled);
    assert_eq!(config.stale_device_days, 90);
    assert!(!config.grace_period_cron.is_empty());
    assert!(!config.license_expiration_cron.is_empty());
    assert!(!config.stale_device_cron.is_empty());
}
