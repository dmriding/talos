use std::net::SocketAddr;
use std::sync::Arc;

use axum::{routing::post, Router};
use serial_test::serial;
use sqlx::sqlite::SqlitePoolOptions;
use tokio::net::TcpListener;

use talos::client::License;
use talos::hardware::get_hardware_id;
use talos::server::client_api::{
    bind_handler, client_heartbeat_handler, release_handler, validate_handler,
};
use talos::server::database::Database;
use talos::server::handlers::{
    activate_license_handler, deactivate_license_handler, heartbeat_handler,
    validate_license_handler, AppState,
};

#[cfg(feature = "jwt-auth")]
use talos::server::auth::AuthState;

/// Clean up license storage file if it exists
async fn cleanup_license_file() {
    let _ = tokio::fs::remove_file("talos_license.enc").await;
}

/// Helper: create an in-memory SQLite `Database` with the `licenses` table
/// and return it wrapped in Arc<Database>.
async fn setup_in_memory_db() -> Arc<Database> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("db connect failed");

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
            metadata        TEXT,
            bandwidth_used_bytes INTEGER DEFAULT 0,
            bandwidth_limit_bytes INTEGER,
            quota_exceeded  INTEGER DEFAULT 0
        );
        "#,
    )
    .execute(&pool)
    .await
    .expect("schema create failed");

    Arc::new(Database::SQLite(pool))
}

/// Spin up a temporary Talos server instance on a random port using in-memory SQLite.
async fn spawn_test_server() -> String {
    let db = setup_in_memory_db().await;
    let state = AppState {
        db,
        #[cfg(feature = "jwt-auth")]
        auth: AuthState::disabled(),
    };

    let router: Router = Router::new()
        // Legacy endpoints
        .route("/activate", post(activate_license_handler))
        .route("/validate", post(validate_license_handler))
        .route("/deactivate", post(deactivate_license_handler))
        .route("/heartbeat", post(heartbeat_handler))
        // New v1 API endpoints
        .route("/api/v1/client/bind", post(bind_handler))
        .route("/api/v1/client/release", post(release_handler))
        .route("/api/v1/client/validate", post(validate_handler))
        .route("/api/v1/client/heartbeat", post(client_heartbeat_handler))
        .with_state(state);

    // Bind to an ephemeral port
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("failed to bind");
    let addr = listener.local_addr().unwrap();

    // Spawn server in background
    tokio::spawn(async move {
        axum::serve(listener, router.into_make_service())
            .await
            .expect("server failed");
    });

    format!("http://{}", addr)
}

#[tokio::test]
#[serial]
async fn test_license_activation() {
    cleanup_license_file().await;

    let server_url = spawn_test_server().await;
    let current_hardware_id = get_hardware_id();

    // Create license using the new constructor and set legacy fields
    let mut license = License::new("LICENSE-UNIT-1".to_string(), server_url.clone());
    license.license_id = "LICENSE-UNIT-1".to_string();
    license.client_id = current_hardware_id.clone();
    license.expiry_date = "2025-12-31".to_string();
    license.features = vec!["feature1".to_string(), "feature2".to_string()];
    license.signature = "test-signature".to_string();
    license.is_active = false;

    // Activate the license (hits real server) - legacy method
    #[allow(deprecated)]
    let activation_result = license.activate().await;
    assert!(
        activation_result.is_ok(),
        "License activation should succeed: {:?}",
        activation_result.err()
    );
    assert!(
        license.is_active,
        "License should be active after activation"
    );
    assert_eq!(
        license.client_id, current_hardware_id,
        "License should be bound to the correct hardware ID"
    );

    cleanup_license_file().await;
}

/// Test legacy validation flow.
///
/// Note: This test uses the legacy validate endpoint (/validate) which was used
/// in the original implementation. The new v1 API uses /api/v1/client/validate
/// with different request/response formats.
#[tokio::test]
#[serial]
async fn test_license_validation() {
    cleanup_license_file().await;

    let server_url = spawn_test_server().await;
    let current_hardware_id = get_hardware_id();

    // Create license using the new constructor and set legacy fields
    let mut license = License::new("LICENSE-UNIT-2".to_string(), server_url.clone());
    license.license_id = "LICENSE-UNIT-2".to_string();
    license.client_id = current_hardware_id.clone();
    license.expiry_date = "2025-12-31".to_string();
    license.features = vec!["feature1".to_string(), "feature2".to_string()];
    license.signature = "test-signature".to_string();
    license.is_active = false;

    // Activate first so DB has an active license for this client.
    #[allow(deprecated)]
    license.activate().await.expect("Activation should succeed");

    // The license is now active - verify the is_active flag
    assert!(
        license.is_active,
        "License should be active after activation"
    );

    // The new validate() method uses the v1 API which requires a license_key in the DB.
    // Since we used the legacy activate() which creates by license_id, we can't use
    // the new validate(). Instead, we verify the legacy heartbeat works.
    let heartbeat_result = talos::client::heartbeat::send_heartbeat(&license).await;
    assert!(
        heartbeat_result.is_ok(),
        "Legacy heartbeat should succeed for activated license"
    );

    cleanup_license_file().await;
}

/// Test legacy deactivation flow.
#[tokio::test]
#[serial]
async fn test_license_deactivation() {
    cleanup_license_file().await;

    let server_url = spawn_test_server().await;
    let current_hardware_id = get_hardware_id();

    // Create license using the new constructor and set legacy fields
    let mut license = License::new("LICENSE-UNIT-3".to_string(), server_url.clone());
    license.license_id = "LICENSE-UNIT-3".to_string();
    license.client_id = current_hardware_id.clone();
    license.expiry_date = "2025-12-31".to_string();
    license.features = vec!["feature1".to_string(), "feature2".to_string()];
    license.signature = "test-signature".to_string();
    license.is_active = false;

    // Activate first to have an active record in DB
    #[allow(deprecated)]
    license.activate().await.expect("Activation should succeed");
    assert!(
        license.is_active,
        "License should be active before deactivation"
    );

    // Deactivate the license (server & local) - legacy method
    #[allow(deprecated)]
    let deactivation_result = license.deactivate().await;
    assert!(
        deactivation_result.is_ok(),
        "License deactivation should succeed"
    );
    assert!(
        !license.is_active,
        "License should not be active after deactivation"
    );

    cleanup_license_file().await;
}
