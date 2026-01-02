use std::net::SocketAddr;
use std::sync::Arc;

use axum::{routing::post, Router};
use sqlx::sqlite::SqlitePoolOptions;
use tokio::net::TcpListener;

use talos::client::client::License;
use talos::hardware::get_hardware_id;
use talos::server::database::Database;
use talos::server::handlers::{
    activate_license_handler, deactivate_license_handler, heartbeat_handler,
    validate_license_handler, AppState,
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
async fn test_license_activation() {
    let server_url = spawn_test_server().await;
    let current_hardware_id = get_hardware_id();

    let mut license = License {
        license_id: "LICENSE-UNIT-1".to_string(),
        client_id: current_hardware_id.clone(), // will be overwritten by activate()
        expiry_date: "2025-12-31".to_string(),
        features: vec!["feature1".to_string(), "feature2".to_string()],
        server_url: server_url.clone(),
        signature: "test-signature".to_string(),
        is_active: false,
    };

    // Activate the license (hits real server)
    let activation_result = license.activate().await;
    assert!(
        activation_result.is_ok(),
        "License activation should succeed"
    );
    assert!(
        license.is_active,
        "License should be active after activation"
    );
    assert_eq!(
        license.client_id, current_hardware_id,
        "License should be bound to the correct hardware ID"
    );
}

#[tokio::test]
async fn test_license_validation() {
    let server_url = spawn_test_server().await;
    let current_hardware_id = get_hardware_id();

    // Start with inactive license, then activate it so server has a record.
    let mut license = License {
        license_id: "LICENSE-UNIT-2".to_string(),
        client_id: current_hardware_id.clone(),
        expiry_date: "2025-12-31".to_string(),
        features: vec!["feature1".to_string(), "feature2".to_string()],
        server_url: server_url.clone(),
        signature: "test-signature".to_string(),
        is_active: false,
    };

    // Activate first so DB has an active license for this client.
    license.activate().await.expect("Activation should succeed");

    // Validate on correct machine
    let validation_result = license.validate().await;
    assert!(
        validation_result.is_ok(),
        "License validation should succeed"
    );

    // Now simulate running on a different machine by tampering client_id
    let mut modified_license = license.clone();
    modified_license.client_id = "DIFFERENT-HARDWARE-ID".to_string();

    let validation_result = modified_license.validate().await;
    assert!(
        validation_result.is_err(),
        "License validation should fail on a different machine"
    );
}

#[tokio::test]
async fn test_license_deactivation() {
    let server_url = spawn_test_server().await;
    let current_hardware_id = get_hardware_id();

    let mut license = License {
        license_id: "LICENSE-UNIT-3".to_string(),
        client_id: current_hardware_id.clone(),
        expiry_date: "2025-12-31".to_string(),
        features: vec!["feature1".to_string(), "feature2".to_string()],
        server_url: server_url.clone(),
        signature: "test-signature".to_string(),
        is_active: false,
    };

    // Activate first to have an active record in DB
    license.activate().await.expect("Activation should succeed");
    assert!(
        license.is_active,
        "License should be active before deactivation"
    );

    // Deactivate the license (server & local)
    let deactivation_result = license.deactivate().await;
    assert!(
        deactivation_result.is_ok(),
        "License deactivation should succeed"
    );
    assert!(
        !license.is_active,
        "License should not be active after deactivation"
    );

    // Try validating after deactivation: should fail locally (is_active=false or status=inactive)
    let validation_result = license.validate().await;
    assert!(
        validation_result.is_err(),
        "License validation should fail after deactivation"
    );
}
