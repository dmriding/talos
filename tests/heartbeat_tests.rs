use std::sync::Arc;

use axum::{extract::State, Json};
use chrono::Utc;
use sqlx::sqlite::SqlitePoolOptions;

use talos::errors::{LicenseError, LicenseResult};
use talos::server::database::{Database, License};
use talos::server::handlers::{heartbeat_handler, AppState, HeartbeatRequest, HeartbeatResponse};

/// Helper: create an in-memory SQLite `Database` with the `licenses` table
/// and return it as Arc<Database>.
async fn setup_in_memory_db() -> LicenseResult<Arc<Database>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .map_err(|e| LicenseError::ServerError(format!("db connect failed: {e}")))?;

    // Minimal schema matching `server::database::License`
    sqlx::query(
        r#"
        CREATE TABLE licenses (
            license_id      TEXT PRIMARY KEY,
            client_id       TEXT NOT NULL,
            status          TEXT NOT NULL,
            features        TEXT,
            issued_at       TEXT NOT NULL,
            expires_at      TEXT,
            hardware_id     TEXT,
            signature       TEXT,
            last_heartbeat  TEXT
        );
        "#,
    )
    .execute(&pool)
    .await
    .map_err(|e| LicenseError::ServerError(format!("schema create failed: {e}")))?;

    Ok(Arc::new(Database::SQLite(pool)))
}

/// Seed a single active license into the DB so heartbeat has something to update.
async fn insert_active_license(
    db: &Database,
    license_id: &str,
    client_id: &str,
) -> LicenseResult<()> {
    let now = Utc::now().naive_utc();

    let license = License {
        license_id: license_id.to_string(),
        client_id: client_id.to_string(),
        status: "active".to_string(),
        features: None,
        issued_at: now,
        expires_at: None,
        hardware_id: None,
        signature: None,
        last_heartbeat: None,
    };

    db.insert_license(license).await?;
    Ok(())
}

#[tokio::test]
async fn valid_heartbeat_updates_license() -> LicenseResult<()> {
    let db = setup_in_memory_db().await?;
    let state = AppState { db: db.clone() };

    let license_id = "HB-LICENSE-1";
    let client_id = "HB-CLIENT-1";

    insert_active_license(&db, license_id, client_id).await?;

    // Call the handler
    let req = HeartbeatRequest {
        license_id: license_id.to_string(),
        client_id: client_id.to_string(),
    };

    let Json(HeartbeatResponse { success }) =
        heartbeat_handler(State(state.clone()), Json(req)).await?;

    assert!(
        success,
        "heartbeat should succeed for valid license/client pair"
    );

    // Verify last_heartbeat was updated
    let stored = db
        .get_license(license_id)
        .await?
        .expect("license should exist");

    assert!(
        stored.last_heartbeat.is_some(),
        "last_heartbeat should be updated after heartbeat"
    );

    Ok(())
}

#[tokio::test]
async fn invalid_heartbeat_fails() -> LicenseResult<()> {
    let db = setup_in_memory_db().await?;
    let state = AppState { db };

    // No license inserted at all
    let req = HeartbeatRequest {
        license_id: "NON_EXISTENT".to_string(),
        client_id: "BAD_CLIENT".to_string(),
    };

    let Json(HeartbeatResponse { success }) = heartbeat_handler(State(state), Json(req)).await?;

    assert!(
        !success,
        "heartbeat should fail for non-existent license/client pair"
    );

    Ok(())
}
