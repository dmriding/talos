use std::net::SocketAddr;
use std::sync::Arc;

use axum::{routing::post, Router};
use sqlx::sqlite::SqlitePoolOptions;
use tokio::net::TcpListener;

use talos::client::License;
use talos::hardware::get_hardware_id;
#[cfg(feature = "admin-api")]
use talos::server::client_api::validate_feature_handler;
use talos::server::client_api::{
    bind_handler, client_heartbeat_handler, release_handler, validate_handler,
};
use talos::server::database::Database;
use talos::server::handlers::{
    activate_license_handler, deactivate_license_handler, heartbeat_handler,
    validate_license_handler, AppState,
};

#[cfg(feature = "admin-api")]
use talos::server::create_license_handler;

#[cfg(feature = "jwt-auth")]
use talos::server::auth::AuthState;

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

/// Spin up a test server with admin API endpoints for v1 API testing.
#[cfg(feature = "admin-api")]
async fn spawn_full_test_server() -> String {
    let db = setup_in_memory_db().await;
    let state = AppState {
        db,
        #[cfg(feature = "jwt-auth")]
        auth: AuthState::disabled(),
    };

    let router: Router = Router::new()
        // Admin API endpoints
        .route("/api/v1/licenses", post(create_license_handler))
        // Client v1 API endpoints
        .route("/api/v1/client/bind", post(bind_handler))
        .route("/api/v1/client/release", post(release_handler))
        .route("/api/v1/client/validate", post(validate_handler))
        .route("/api/v1/client/heartbeat", post(client_heartbeat_handler))
        .route(
            "/api/v1/client/validate-feature",
            post(validate_feature_handler),
        )
        // Legacy endpoints (for backwards compatibility)
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

/// Test the legacy API flow: activate -> heartbeat -> deactivate
///
/// Note: The new v1 API (bind/validate/release) requires a pre-existing license
/// in the database, which is normally created via the admin API. This test uses
/// the legacy flow which is self-contained and creates the license on activation.
#[tokio::test]
async fn integration_test_license_lifecycle() {
    // Start a real HTTP server backed by in-memory SQLite
    let server_url = spawn_test_server().await;

    let hardware_id = get_hardware_id();

    // Create license using the new constructor and set legacy fields
    let mut license = License::new("LICENSE-12345".to_string(), server_url.clone());
    license.license_id = "LICENSE-12345".to_string();
    license.client_id = hardware_id.clone();
    license.expiry_date = "2025-12-31".to_string();
    license.features = vec!["feature1".to_string(), "feature2".to_string()];
    license.signature = "test-signature".to_string();
    license.is_active = false;

    // --- ACTIVATE (legacy method - creates the license record) ---
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

    // --- LEGACY HEARTBEAT ---
    // Use the legacy heartbeat module which uses the old endpoint
    let heartbeat_result = talos::client::heartbeat::send_heartbeat(&license).await;
    assert!(
        heartbeat_result.is_ok(),
        "Heartbeat should succeed: {:?}",
        heartbeat_result.err()
    );

    // --- DEACTIVATE (legacy method) ---
    #[allow(deprecated)]
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

/// Test the new v1 API flow: create (admin) -> bind -> validate -> heartbeat -> release
///
/// This test requires the `admin-api` feature to create a license first.
#[cfg(feature = "admin-api")]
#[tokio::test]
async fn integration_test_v1_api_lifecycle() {
    use serde_json::json;

    // Start a server with both admin and client APIs
    let server_url = spawn_full_test_server().await;
    let client = reqwest::Client::new();

    // Step 1: Create a license via admin API
    let create_response = client
        .post(format!("{}/api/v1/licenses", server_url))
        .json(&json!({
            "org_id": "test-org",
            "features": ["feature_a", "feature_b"],
            "expires_at": "2030-12-31T23:59:59Z"
        }))
        .send()
        .await
        .expect("create request failed");

    assert!(
        create_response.status().is_success(),
        "License creation should succeed: {}",
        create_response.status()
    );

    let create_body: serde_json::Value = create_response.json().await.expect("parse json failed");
    let license_key = create_body["license_key"]
        .as_str()
        .expect("license_key missing");

    // Step 2: Create client License and bind
    let mut license = License::new(license_key.to_string(), server_url.clone());

    let bind_result = license
        .bind(Some("Test Workstation"), Some("Test Device"))
        .await;
    assert!(
        bind_result.is_ok(),
        "Bind should succeed: {:?}",
        bind_result.err()
    );

    let bind_data = bind_result.unwrap();
    assert!(
        bind_data.features.contains(&"feature_a".to_string()),
        "Should have feature_a"
    );
    assert!(
        bind_data.features.contains(&"feature_b".to_string()),
        "Should have feature_b"
    );

    // Step 3: Validate the license
    let validate_result = license.validate().await;
    assert!(
        validate_result.is_ok(),
        "Validate should succeed: {:?}",
        validate_result.err()
    );

    let validation = validate_result.unwrap();
    assert!(validation.has_feature("feature_a"));
    assert!(validation.has_feature("feature_b"));
    assert!(!validation.has_feature("feature_c"));

    // Step 4: Heartbeat
    let heartbeat_result = license.heartbeat().await;
    assert!(
        heartbeat_result.is_ok(),
        "Heartbeat should succeed: {:?}",
        heartbeat_result.err()
    );

    let heartbeat = heartbeat_result.unwrap();
    assert!(
        !heartbeat.server_time.is_empty(),
        "Server time should be returned"
    );

    // Step 5: Validate specific feature
    let feature_result = license.validate_feature("feature_a").await;
    assert!(
        feature_result.is_ok(),
        "Feature validation should succeed: {:?}",
        feature_result.err()
    );
    assert!(
        feature_result.unwrap().allowed,
        "feature_a should be allowed"
    );

    // Missing feature returns an error (403 FEATURE_NOT_INCLUDED)
    let feature_result = license.validate_feature("feature_c").await;
    assert!(
        feature_result.is_err(),
        "Feature validation should return error for missing feature"
    );

    // Step 6: Release the license
    let release_result = license.release().await;
    assert!(
        release_result.is_ok(),
        "Release should succeed: {:?}",
        release_result.err()
    );

    // Verify license is no longer bound - validate should fail
    let validate_after_release = license.validate().await;
    assert!(
        validate_after_release.is_err(),
        "Validate should fail after release"
    );
}
