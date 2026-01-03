use std::sync::Arc;

use chrono::Utc;
use sqlx::sqlite::SqlitePoolOptions;

use talos::errors::{LicenseError, LicenseResult};
use talos::server::database::{BindingAction, Database, License, PerformedBy};

/// Helper: create an in-memory SQLite Database with both tables.
async fn setup_in_memory_db() -> LicenseResult<Arc<Database>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .map_err(|e| LicenseError::ServerError(format!("db connect failed: {e}")))?;

    // Licenses table with all extended fields
    sqlx::query(
        r#"
        CREATE TABLE licenses (
            license_id      TEXT PRIMARY KEY,
            client_id       TEXT,
            status          TEXT NOT NULL,
            features        TEXT,
            issued_at       TEXT NOT NULL,
            expires_at      TEXT,
            hardware_id     TEXT,
            signature       TEXT,
            last_heartbeat  TEXT,
            org_id          TEXT,
            org_name        TEXT,
            license_key     TEXT UNIQUE,
            tier            TEXT,
            device_name     TEXT,
            device_info     TEXT,
            bound_at        TEXT,
            last_seen_at    TEXT,
            suspended_at    TEXT,
            revoked_at      TEXT,
            revoke_reason   TEXT,
            grace_period_ends_at TEXT,
            suspension_message TEXT,
            is_blacklisted  INTEGER DEFAULT 0,
            blacklisted_at  TEXT,
            blacklist_reason TEXT,
            metadata        TEXT,
            bandwidth_used_bytes INTEGER DEFAULT 0,
            bandwidth_limit_bytes INTEGER,
            quota_exceeded  INTEGER DEFAULT 0
        );
        "#,
    )
    .execute(&pool)
    .await
    .map_err(|e| LicenseError::ServerError(format!("licenses table create failed: {e}")))?;

    // Binding history table
    sqlx::query(
        r#"
        CREATE TABLE license_binding_history (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            license_id      TEXT NOT NULL,
            action          TEXT NOT NULL,
            hardware_id     TEXT,
            device_name     TEXT,
            device_info     TEXT,
            performed_by    TEXT,
            reason          TEXT,
            created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(&pool)
    .await
    .map_err(|e| LicenseError::ServerError(format!("history table create failed: {e}")))?;

    Ok(Arc::new(Database::SQLite(pool)))
}

/// Helper: insert a license with specific fields for testing.
async fn insert_test_license(
    db: &Database,
    license_id: &str,
    license_key: Option<&str>,
    org_id: Option<&str>,
) -> LicenseResult<()> {
    let now = Utc::now().naive_utc();

    let license = License {
        license_id: license_id.to_string(),
        client_id: None,
        status: "active".to_string(),
        features: None,
        issued_at: now,
        expires_at: None,
        hardware_id: None,
        signature: None,
        last_heartbeat: None,
        org_id: org_id.map(String::from),
        org_name: None,
        license_key: license_key.map(String::from),
        tier: None,
        device_name: None,
        device_info: None,
        bound_at: None,
        last_seen_at: None,
        suspended_at: None,
        revoked_at: None,
        revoke_reason: None,
        grace_period_ends_at: None,
        suspension_message: None,
        is_blacklisted: None,
        blacklisted_at: None,
        blacklist_reason: None,
        metadata: None,
        bandwidth_used_bytes: None,
        bandwidth_limit_bytes: None,
        quota_exceeded: None,
    };

    db.insert_license(license).await
}

// =============================================================================
// License Key Tests
// =============================================================================

#[tokio::test]
async fn get_license_by_key_returns_license() -> LicenseResult<()> {
    let db = setup_in_memory_db().await?;

    insert_test_license(&db, "LIC-001", Some("XXXX-YYYY-ZZZZ"), None).await?;

    let found = db.get_license_by_key("XXXX-YYYY-ZZZZ").await?;
    assert!(found.is_some(), "should find license by key");
    assert_eq!(found.unwrap().license_id, "LIC-001");

    Ok(())
}

#[tokio::test]
async fn get_license_by_key_returns_none_for_missing() -> LicenseResult<()> {
    let db = setup_in_memory_db().await?;

    let found = db.get_license_by_key("NONEXISTENT-KEY").await?;
    assert!(found.is_none(), "should return None for missing key");

    Ok(())
}

#[tokio::test]
async fn license_key_exists_returns_true_when_present() -> LicenseResult<()> {
    let db = setup_in_memory_db().await?;

    insert_test_license(&db, "LIC-002", Some("KEY-EXISTS"), None).await?;

    let exists = db.license_key_exists("KEY-EXISTS").await?;
    assert!(exists, "should return true for existing key");

    Ok(())
}

#[tokio::test]
async fn license_key_exists_returns_false_when_missing() -> LicenseResult<()> {
    let db = setup_in_memory_db().await?;

    let exists = db.license_key_exists("NO-SUCH-KEY").await?;
    assert!(!exists, "should return false for missing key");

    Ok(())
}

// =============================================================================
// Organization Tests
// =============================================================================

#[tokio::test]
async fn list_licenses_by_org_returns_matching() -> LicenseResult<()> {
    let db = setup_in_memory_db().await?;

    // Insert licenses for two orgs
    insert_test_license(&db, "LIC-ORG1-A", None, Some("org-001")).await?;
    insert_test_license(&db, "LIC-ORG1-B", None, Some("org-001")).await?;
    insert_test_license(&db, "LIC-ORG2-A", None, Some("org-002")).await?;

    let org1_licenses = db.list_licenses_by_org("org-001").await?;
    assert_eq!(org1_licenses.len(), 2, "org-001 should have 2 licenses");

    let org2_licenses = db.list_licenses_by_org("org-002").await?;
    assert_eq!(org2_licenses.len(), 1, "org-002 should have 1 license");

    let org3_licenses = db.list_licenses_by_org("org-003").await?;
    assert_eq!(org3_licenses.len(), 0, "org-003 should have 0 licenses");

    Ok(())
}

// =============================================================================
// Status Update Tests
// =============================================================================

#[tokio::test]
async fn update_license_status_changes_status() -> LicenseResult<()> {
    let db = setup_in_memory_db().await?;

    insert_test_license(&db, "LIC-STATUS", None, None).await?;

    // Verify initial status
    let license = db.get_license("LIC-STATUS").await?.unwrap();
    assert_eq!(license.status, "active");

    // Update to suspended
    let updated = db.update_license_status("LIC-STATUS", "suspended").await?;
    assert!(updated, "should return true on successful update");

    // Verify new status
    let license = db.get_license("LIC-STATUS").await?.unwrap();
    assert_eq!(license.status, "suspended");

    Ok(())
}

#[tokio::test]
async fn update_license_status_returns_false_for_missing() -> LicenseResult<()> {
    let db = setup_in_memory_db().await?;

    let updated = db.update_license_status("NONEXISTENT", "active").await?;
    assert!(!updated, "should return false for missing license");

    Ok(())
}

// =============================================================================
// Hardware Binding Tests
// =============================================================================

#[tokio::test]
async fn bind_license_sets_hardware_fields() -> LicenseResult<()> {
    let db = setup_in_memory_db().await?;

    insert_test_license(&db, "LIC-BIND", None, None).await?;

    // Verify not bound initially
    let license = db.get_license("LIC-BIND").await?.unwrap();
    assert!(!license.is_bound(), "should not be bound initially");

    // Bind to hardware
    let bound = db
        .bind_license(
            "LIC-BIND",
            "HW-12345",
            Some("Developer Workstation"),
            Some("Windows 11, Intel i7"),
        )
        .await?;
    assert!(bound, "should return true on successful bind");

    // Verify binding
    let license = db.get_license("LIC-BIND").await?.unwrap();
    assert!(license.is_bound(), "should be bound after bind_license");
    assert_eq!(license.hardware_id, Some("HW-12345".to_string()));
    assert_eq!(
        license.device_name,
        Some("Developer Workstation".to_string())
    );
    assert!(license.bound_at.is_some(), "bound_at should be set");
    assert!(license.last_seen_at.is_some(), "last_seen_at should be set");

    Ok(())
}

#[tokio::test]
async fn release_license_clears_hardware_fields() -> LicenseResult<()> {
    let db = setup_in_memory_db().await?;

    insert_test_license(&db, "LIC-RELEASE", None, None).await?;

    // Bind first
    db.bind_license("LIC-RELEASE", "HW-99999", None, None)
        .await?;
    let license = db.get_license("LIC-RELEASE").await?.unwrap();
    assert!(license.is_bound(), "should be bound after bind");

    // Release
    let released = db.release_license("LIC-RELEASE").await?;
    assert!(released, "should return true on successful release");

    // Verify release
    let license = db.get_license("LIC-RELEASE").await?.unwrap();
    assert!(!license.is_bound(), "should not be bound after release");
    assert!(license.hardware_id.is_none(), "hardware_id should be None");
    assert!(license.device_name.is_none(), "device_name should be None");
    assert!(license.bound_at.is_none(), "bound_at should be None");

    Ok(())
}

// =============================================================================
// Binding History Tests
// =============================================================================

#[tokio::test]
async fn record_binding_history_inserts_record() -> LicenseResult<()> {
    let db = setup_in_memory_db().await?;

    insert_test_license(&db, "LIC-HISTORY", None, None).await?;

    // Record a bind action
    db.record_binding_history(
        "LIC-HISTORY",
        BindingAction::Bind,
        Some("HW-HIST-001"),
        Some("Test Device"),
        Some("Test Info"),
        PerformedBy::Client,
        None,
    )
    .await?;

    // Record a release action
    db.record_binding_history(
        "LIC-HISTORY",
        BindingAction::AdminRelease,
        Some("HW-HIST-001"),
        None,
        None,
        PerformedBy::Admin,
        Some("User requested transfer"),
    )
    .await?;

    // Verify records were inserted (query directly since we don't have a get method yet)
    let pool = match db.as_ref() {
        Database::SQLite(p) => p,
        #[allow(unreachable_patterns)]
        _ => panic!("Expected SQLite"),
    };

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM license_binding_history WHERE license_id = ?")
            .bind("LIC-HISTORY")
            .fetch_one(pool)
            .await
            .map_err(|e| LicenseError::ServerError(format!("query failed: {e}")))?;

    assert_eq!(count.0, 2, "should have 2 history records");

    Ok(())
}

// =============================================================================
// Last Seen Tests
// =============================================================================

#[tokio::test]
async fn update_last_seen_sets_timestamp() -> LicenseResult<()> {
    let db = setup_in_memory_db().await?;

    insert_test_license(&db, "LIC-SEEN", None, None).await?;

    // Verify initially None
    let license = db.get_license("LIC-SEEN").await?.unwrap();
    assert!(license.last_seen_at.is_none(), "should be None initially");

    // Update last seen
    let updated = db.update_last_seen("LIC-SEEN").await?;
    assert!(updated, "should return true on successful update");

    // Verify timestamp set
    let license = db.get_license("LIC-SEEN").await?.unwrap();
    assert!(
        license.last_seen_at.is_some(),
        "last_seen_at should be set after update"
    );

    Ok(())
}

// =============================================================================
// License Helper Method Tests
// =============================================================================

#[tokio::test]
async fn license_is_valid_checks_all_conditions() -> LicenseResult<()> {
    let _db = setup_in_memory_db().await?;
    let now = Utc::now().naive_utc();

    // Valid license: active, not expired, not blacklisted
    let valid_license = License {
        license_id: "VALID".to_string(),
        client_id: None,
        status: "active".to_string(),
        features: None,
        issued_at: now,
        expires_at: None,
        hardware_id: None,
        signature: None,
        last_heartbeat: None,
        org_id: None,
        org_name: None,
        license_key: None,
        tier: None,
        device_name: None,
        device_info: None,
        bound_at: None,
        last_seen_at: None,
        suspended_at: None,
        revoked_at: None,
        revoke_reason: None,
        grace_period_ends_at: None,
        suspension_message: None,
        is_blacklisted: None,
        blacklisted_at: None,
        blacklist_reason: None,
        metadata: None,
        bandwidth_used_bytes: None,
        bandwidth_limit_bytes: None,
        quota_exceeded: None,
    };
    assert!(valid_license.is_valid(), "active license should be valid");

    // Inactive license
    let mut inactive = valid_license.clone();
    inactive.status = "inactive".to_string();
    assert!(!inactive.is_valid(), "inactive license should not be valid");

    // Blacklisted license
    let mut blacklisted = valid_license.clone();
    blacklisted.is_blacklisted = Some(true);
    assert!(
        !blacklisted.is_valid(),
        "blacklisted license should not be valid"
    );

    // Expired license
    let mut expired = valid_license.clone();
    expired.expires_at = Some(now - chrono::Duration::days(1));
    assert!(!expired.is_valid(), "expired license should not be valid");

    Ok(())
}
