use std::net::SocketAddr;
use std::sync::Arc;

use axum::{routing::post, Router};
use sqlx::sqlite::SqlitePoolOptions;
use tokio::net::TcpListener;

use talos::client::client::License;
use talos::hardware::get_hardware_id;
use talos::server::database::Database;
use talos::server::handlers::{
    activate_license_handler,
    deactivate_license_handler,
    heartbeat_handler,
    validate_license_handler,
    AppState,
};

/// Helper: create an in-memory SQLite `Database` with the `licenses` table
/// and return it wrapped in Arc<Database>.
async fn setup_in_memory_db() -> Arc<Database> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("db connect failed");

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
    .expect("schema create failed");

    Arc::new(Database::SQLite(pool))
}

/// Spin up a temporary Talos server instance on a random port using in-memory SQLite.
async fn spawn_test_server() -> String {
    let db = setup_in_memory_db().await;
    let state = AppState { db };

    let router: Router = Router::new()
        .route("/activate", post(activate_license_handler))
        .route("/validate", post(validate_license_handler))
        .route("/deactivate", post(deactivate_license_handler))
        .route("/heartbeat", post(heartbeat_handler))
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
async fn integration_test_license_lifecycle() {
    // Start a real HTTP server backed by in-memory SQLite
    let server_url = spawn_test_server().await;

    let hardware_id = get_hardware_id();

    let mut license = License {
        license_id: "LICENSE-12345".to_string(),
        // Will be overwritten by `activate`, but we start with the current hardware ID.
        client_id: hardware_id.clone(),
        expiry_date: "2025-12-31".to_string(),
        features: vec!["feature1".to_string(), "feature2".to_string()],
        server_url: server_url.clone(),
        signature: "test-signature".to_string(),
        is_active: false,
    };

    // --- ACTIVATE ---
    let activation_result = license.activate().await;
    assert!(
        activation_result.is_ok(),
        "License activation should succeed: {:?}",
        activation_result.err()
    );
    assert!(license.is_active, "License should be active after activation");

    // --- VALIDATE ---
    let validation_result = license.validate().await;
    assert!(
        validation_result.is_ok(),
        "License validation should succeed: {:?}",
        validation_result.err()
    );

    // --- HEARTBEAT ---
    let hb_result = license.heartbeat().await;
    assert!(
        hb_result.unwrap_or(false),
        "Heartbeat should succeed and return true"
    );

    // --- DEACTIVATE ---
    let deactivation_result = license.deactivate().await;
    assert!(
        deactivation_result.is_ok(),
        "License deactivation should succeed: {:?}",
        deactivation_result.err()
    );
    assert!(
        !license.is_active,
        "License should not be active after deactivation"
    );
}
