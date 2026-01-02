//! Integration tests for the Admin API endpoints.
//!
//! These tests require the `admin-api` feature to be enabled.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use talos::server::database::Database;
use talos::server::handlers::AppState;
use talos::server::routes::build_router;
use tower::ServiceExt;

/// Helper to create a test database and app state.
async fn setup_test_app() -> AppState {
    // Use an in-memory SQLite database for testing
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
        }
        #[cfg(feature = "postgres")]
        Database::Postgres(_) => {
            panic!("PostgreSQL not supported in tests");
        }
    }

    AppState { db }
}

/// Helper to make a JSON request to the app.
async fn json_request(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let body_bytes = body
        .map(|v| serde_json::to_vec(&v).unwrap())
        .unwrap_or_default();

    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(body_bytes))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap_or(json!({}));

    (status, body)
}

#[tokio::test]
async fn create_license_returns_created() {
    let state = setup_test_app().await;
    let app = build_router(state);

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "org-123",
            "org_name": "Test Organization",
            "features": ["feature_a", "feature_b"],
            "expires_at": "2025-12-31"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.get("license_id").is_some());
    assert!(body.get("license_key").is_some());
    assert_eq!(body["status"], "active");
    assert_eq!(body["org_id"], "org-123");
    assert_eq!(body["org_name"], "Test Organization");
    assert_eq!(body["features"], json!(["feature_a", "feature_b"]));
}

#[tokio::test]
async fn create_license_minimal_request() {
    let state = setup_test_app().await;
    let app = build_router(state);

    // Minimal request with just features
    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "features": ["basic"]
        })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.get("license_id").is_some());
    assert!(body.get("license_key").is_some());
    assert_eq!(body["status"], "active");
    assert!(body["org_id"].is_null());
}

#[tokio::test]
async fn get_license_returns_license() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // First create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "org-456",
            "features": ["feature_x"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Now get it
    let app = build_router(state);
    let (status, body) = json_request(
        app,
        "GET",
        &format!("/api/v1/licenses/{}", license_id),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["license_id"], license_id);
    assert_eq!(body["org_id"], "org-456");
}

#[tokio::test]
async fn get_license_not_found() {
    let state = setup_test_app().await;
    let app = build_router(state);

    let (status, body) = json_request(app, "GET", "/api/v1/licenses/nonexistent-id", None).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn list_licenses_by_org() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a few licenses for the same org
    for i in 0..3 {
        let _ = json_request(
            app.clone(),
            "POST",
            "/api/v1/licenses",
            Some(json!({
                "org_id": "org-list-test",
                "features": [format!("feature_{}", i)]
            })),
        )
        .await;
    }

    // Create one for a different org
    let _ = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "other-org",
            "features": ["other"]
        })),
    )
    .await;

    // List licenses for org-list-test
    let app = build_router(state);
    let (status, body) =
        json_request(app, "GET", "/api/v1/licenses?org_id=org-list-test", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 3);
    assert_eq!(body["licenses"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn list_licenses_requires_org_id() {
    let state = setup_test_app().await;
    let app = build_router(state);

    let (status, body) = json_request(app, "GET", "/api/v1/licenses", None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("org_id"));
}

#[tokio::test]
async fn update_license_changes_fields() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "features": ["old_feature"],
            "expires_at": "2025-06-30"
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Update it
    let app = build_router(state.clone());
    let (status, body) = json_request(
        app,
        "PATCH",
        &format!("/api/v1/licenses/{}", license_id),
        Some(json!({
            "features": ["new_feature_a", "new_feature_b"],
            "expires_at": "2026-12-31"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["features"], json!(["new_feature_a", "new_feature_b"]));

    // Verify the update persisted
    let app = build_router(state);
    let (_, get_body) = json_request(
        app,
        "GET",
        &format!("/api/v1/licenses/{}", license_id),
        None,
    )
    .await;

    assert_eq!(
        get_body["features"],
        json!(["new_feature_a", "new_feature_b"])
    );
}

#[tokio::test]
async fn update_license_not_found() {
    let state = setup_test_app().await;
    let app = build_router(state);

    let (status, body) = json_request(
        app,
        "PATCH",
        "/api/v1/licenses/nonexistent-id",
        Some(json!({
            "features": ["test"]
        })),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn batch_create_licenses() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/licenses/batch",
        Some(json!({
            "count": 5,
            "org_id": "batch-org",
            "features": ["batch_feature"]
        })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["created"], 5);
    assert_eq!(body["licenses"].as_array().unwrap().len(), 5);

    // Verify all licenses have unique keys
    let licenses = body["licenses"].as_array().unwrap();
    let mut keys: Vec<&str> = licenses
        .iter()
        .map(|l| l["license_key"].as_str().unwrap())
        .collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), 5);

    // Verify licenses exist in database
    let app = build_router(state);
    let (status, list_body) =
        json_request(app, "GET", "/api/v1/licenses?org_id=batch-org", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(list_body["total"], 5);
}

#[tokio::test]
async fn batch_create_rejects_zero_count() {
    let state = setup_test_app().await;
    let app = build_router(state);

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/licenses/batch",
        Some(json!({
            "count": 0,
            "features": ["test"]
        })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("count"));
}

#[tokio::test]
async fn batch_create_rejects_too_many() {
    let state = setup_test_app().await;
    let app = build_router(state);

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/licenses/batch",
        Some(json!({
            "count": 1001,
            "features": ["test"]
        })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("1000"));
}

#[tokio::test]
async fn create_license_with_metadata() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "features": ["feature_a"],
            "metadata": {
                "customer_email": "test@example.com",
                "purchase_date": "2024-01-15",
                "order_id": 12345
            }
        })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body["metadata"].is_object());
    assert_eq!(body["metadata"]["customer_email"], "test@example.com");
    assert_eq!(body["metadata"]["order_id"], 12345);

    // Verify metadata persisted
    let license_id = body["license_id"].as_str().unwrap();
    let app = build_router(state);
    let (_, get_body) = json_request(
        app,
        "GET",
        &format!("/api/v1/licenses/{}", license_id),
        None,
    )
    .await;

    assert_eq!(get_body["metadata"]["customer_email"], "test@example.com");
}

#[tokio::test]
async fn update_license_metadata() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "features": ["test"],
            "metadata": {"initial": "value"}
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Update metadata
    let app = build_router(state);
    let (status, body) = json_request(
        app,
        "PATCH",
        &format!("/api/v1/licenses/{}", license_id),
        Some(json!({
            "metadata": {"updated": "new_value", "extra": 123}
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["metadata"]["updated"], "new_value");
    assert_eq!(body["metadata"]["extra"], 123);
}

#[tokio::test]
async fn list_licenses_pagination() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create 15 licenses
    for _ in 0..15 {
        let _ = json_request(
            app.clone(),
            "POST",
            "/api/v1/licenses",
            Some(json!({
                "org_id": "pagination-org",
                "features": ["test"]
            })),
        )
        .await;
    }

    // Get first page (10 per page)
    let app = build_router(state.clone());
    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/licenses?org_id=pagination-org&page=1&per_page=10",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 15);
    assert_eq!(body["page"], 1);
    assert_eq!(body["per_page"], 10);
    assert_eq!(body["total_pages"], 2);
    assert_eq!(body["licenses"].as_array().unwrap().len(), 10);

    // Get second page
    let app = build_router(state);
    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/licenses?org_id=pagination-org&page=2&per_page=10",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["licenses"].as_array().unwrap().len(), 5);
}
