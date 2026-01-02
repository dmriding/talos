// tests/server_tests.rs

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
//use chrono::NaiveDateTime;
use sqlx::sqlite::SqlitePoolOptions;

use talos::errors::LicenseResult;
use talos::server::database::Database;
use talos::server::handlers::{
    activate_license_handler, deactivate_license_handler, validate_license_handler, AppState,
    LicenseRequest, LicenseResponse,
};

/// Helper: create an in-memory SQLite Database with the licenses table.
async fn setup_in_memory_db() -> LicenseResult<Arc<Database>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .map_err(|e| talos::errors::LicenseError::ServerError(format!("db connect failed: {e}")))?;

    // Schema matching `server::database::License` with all extended fields
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
            metadata        TEXT
        );
        "#,
    )
    .execute(&pool)
    .await
    .map_err(|e| talos::errors::LicenseError::ServerError(format!("schema create failed: {e}")))?;

    Ok(Arc::new(Database::SQLite(pool)))
}

#[tokio::test]
async fn activate_and_validate_round_trip() -> LicenseResult<()> {
    // Arrange: in-memory DB + AppState
    let db = setup_in_memory_db().await?;
    let state = AppState { db };

    let license_id = "TEST-LICENSE-1".to_string();
    let client_id = "CLIENT-123".to_string();

    // Act: activate license
    let activate_req = LicenseRequest {
        license_id: license_id.clone(),
        client_id: client_id.clone(),
    };

    let Json(LicenseResponse { success }) =
        activate_license_handler(State(state.clone()), Json(activate_req)).await?;
    assert!(success, "activation should succeed");

    // Act: validate with correct client
    let validate_req = LicenseRequest {
        license_id: license_id.clone(),
        client_id: client_id.clone(),
    };

    let Json(LicenseResponse { success }) =
        validate_license_handler(State(state.clone()), Json(validate_req)).await?;
    assert!(success, "validation should succeed for active license");

    // Act: validate with wrong client -> should fail
    let validate_wrong_req = LicenseRequest {
        license_id: license_id.clone(),
        client_id: "OTHER-CLIENT".to_string(),
    };

    let Json(LicenseResponse { success }) =
        validate_license_handler(State(state.clone()), Json(validate_wrong_req)).await?;
    assert!(!success, "validation should fail for wrong client_id");

    Ok(())
}

#[tokio::test]
async fn deactivate_makes_license_invalid() -> LicenseResult<()> {
    // Arrange
    let db = setup_in_memory_db().await?;
    let state = AppState { db };

    let license_id = "TEST-LICENSE-2".to_string();
    let client_id = "CLIENT-XYZ".to_string();

    // Activate first
    let activate_req = LicenseRequest {
        license_id: license_id.clone(),
        client_id: client_id.clone(),
    };
    let Json(LicenseResponse { success }) =
        activate_license_handler(State(state.clone()), Json(activate_req)).await?;
    assert!(success);

    // Deactivate
    let deactivate_req = LicenseRequest {
        license_id: license_id.clone(),
        client_id: client_id.clone(),
    };
    let Json(LicenseResponse { success }) =
        deactivate_license_handler(State(state.clone()), Json(deactivate_req)).await?;
    assert!(success, "deactivation should succeed");

    // Validate after deactivation -> should fail
    let validate_req = LicenseRequest {
        license_id: license_id.clone(),
        client_id: client_id.clone(),
    };
    let Json(LicenseResponse { success }) =
        validate_license_handler(State(state), Json(validate_req)).await?;
    assert!(
        !success,
        "validation should fail after license is deactivated"
    );

    Ok(())
}
