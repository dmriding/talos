//! Integration tests for the Admin API endpoints.
//!
//! These tests require the `admin-api` feature to be enabled.

#![cfg(feature = "admin-api")]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use talos::server::database::Database;
use talos::server::handlers::AppState;
use talos::server::routes::build_router;
use tower::ServiceExt;

#[cfg(feature = "jwt-auth")]
use talos::server::auth::AuthState;

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
                    metadata TEXT,
                    bandwidth_used_bytes INTEGER DEFAULT 0,
                    bandwidth_limit_bytes INTEGER,
                    quota_exceeded INTEGER DEFAULT 0
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

    AppState {
        db,
        #[cfg(feature = "jwt-auth")]
        auth: AuthState::disabled(),
    }
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
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not found"));
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
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("org_id"));
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
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not found"));
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
    assert!(body["error"]["message"].as_str().unwrap().contains("count"));
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
    assert!(body["error"]["message"].as_str().unwrap().contains("1000"));
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

// ============================================================================
// Revoke License Tests
// ============================================================================

#[tokio::test]
async fn revoke_license_immediate() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "revoke-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Revoke immediately (grace_period_days = 0)
    let app = build_router(state.clone());
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/revoke", license_id),
        Some(json!({
            "reason": "Terms of service violation",
            "grace_period_days": 0
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["status"], "revoked");
    assert!(body["grace_period_ends_at"].is_null());

    // Verify the license is revoked
    let app = build_router(state);
    let (_, get_body) = json_request(
        app,
        "GET",
        &format!("/api/v1/licenses/{}", license_id),
        None,
    )
    .await;

    assert_eq!(get_body["status"], "revoked");
}

#[tokio::test]
async fn revoke_license_with_grace_period() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "revoke-grace-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Revoke with 7 day grace period
    let app = build_router(state.clone());
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/revoke", license_id),
        Some(json!({
            "reason": "Non-payment",
            "grace_period_days": 7,
            "message": "Please renew your subscription within 7 days"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["status"], "suspended");
    assert!(!body["grace_period_ends_at"].is_null());

    // Verify the license is suspended with grace period
    let app = build_router(state);
    let (_, get_body) = json_request(
        app,
        "GET",
        &format!("/api/v1/licenses/{}", license_id),
        None,
    )
    .await;

    assert_eq!(get_body["status"], "suspended");
}

#[tokio::test]
async fn revoke_license_not_found() {
    let state = setup_test_app().await;
    let app = build_router(state);

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/licenses/nonexistent-id/revoke",
        Some(json!({
            "reason": "Test",
            "grace_period_days": 0
        })),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not found"));
}

#[tokio::test]
async fn revoke_already_revoked_license() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "double-revoke-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Revoke it
    let app = build_router(state.clone());
    let _ = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/revoke", license_id),
        Some(json!({
            "grace_period_days": 0
        })),
    )
    .await;

    // Try to revoke again
    let app = build_router(state);
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/revoke", license_id),
        Some(json!({
            "grace_period_days": 0
        })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("already revoked"));
}

// ============================================================================
// Reinstate License Tests
// ============================================================================

#[tokio::test]
async fn reinstate_revoked_license() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "reinstate-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Revoke it
    let app = build_router(state.clone());
    let _ = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/revoke", license_id),
        Some(json!({
            "reason": "Test revocation",
            "grace_period_days": 0
        })),
    )
    .await;

    // Reinstate it
    let app = build_router(state.clone());
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/reinstate", license_id),
        Some(json!({
            "reason": "Customer paid outstanding balance"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["status"], "active");
    assert_eq!(body["message"], "License has been reinstated");

    // Verify the license is active again
    let app = build_router(state);
    let (_, get_body) = json_request(
        app,
        "GET",
        &format!("/api/v1/licenses/{}", license_id),
        None,
    )
    .await;

    assert_eq!(get_body["status"], "active");
}

#[tokio::test]
async fn reinstate_suspended_license() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "reinstate-suspended-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Suspend it with grace period
    let app = build_router(state.clone());
    let _ = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/revoke", license_id),
        Some(json!({
            "reason": "Non-payment",
            "grace_period_days": 7,
            "message": "Please pay within 7 days"
        })),
    )
    .await;

    // Reinstate it
    let app = build_router(state.clone());
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/reinstate", license_id),
        Some(json!({
            "reason": "Payment received"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["status"], "active");

    // Verify the license is active and suspension fields cleared
    let app = build_router(state);
    let (_, get_body) = json_request(
        app,
        "GET",
        &format!("/api/v1/licenses/{}", license_id),
        None,
    )
    .await;

    assert_eq!(get_body["status"], "active");
}

#[tokio::test]
async fn reinstate_with_new_expiration() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license with expiration
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "reinstate-expire-org",
            "features": ["test"],
            "expires_at": "2025-06-30"
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Revoke it
    let app = build_router(state.clone());
    let _ = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/revoke", license_id),
        Some(json!({
            "grace_period_days": 0
        })),
    )
    .await;

    // Reinstate with new expiration
    let app = build_router(state.clone());
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/reinstate", license_id),
        Some(json!({
            "new_expires_at": "2026-12-31",
            "reason": "Renewed subscription"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert!(body["expires_at"].as_str().unwrap().contains("2026-12-31"));
}

#[tokio::test]
async fn reinstate_license_not_found() {
    let state = setup_test_app().await;
    let app = build_router(state);

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/licenses/nonexistent-id/reinstate",
        Some(json!({
            "reason": "Test"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not found"));
}

#[tokio::test]
async fn reinstate_already_active_license() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license (already active)
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "reinstate-active-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Try to reinstate an already active license
    let app = build_router(state);
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/reinstate", license_id),
        Some(json!({})),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("already active"));
}

// ============================================================================
// Extend License Tests
// ============================================================================

#[tokio::test]
async fn extend_license_success() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license with expiration
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "extend-org",
            "features": ["test"],
            "expires_at": "2025-06-30"
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Extend the license
    let app = build_router(state.clone());
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/extend", license_id),
        Some(json!({
            "new_expires_at": "2027-12-31",
            "reason": "Customer renewed for 2 years"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert!(body["previous_expires_at"]
        .as_str()
        .unwrap()
        .contains("2025-06-30"));
    assert!(body["new_expires_at"]
        .as_str()
        .unwrap()
        .contains("2027-12-31"));

    // Verify the license was extended
    let app = build_router(state);
    let (_, get_body) = json_request(
        app,
        "GET",
        &format!("/api/v1/licenses/{}", license_id),
        None,
    )
    .await;

    assert!(get_body["expires_at"]
        .as_str()
        .unwrap()
        .contains("2027-12-31"));
}

#[tokio::test]
async fn extend_license_no_previous_expiration() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license without expiration
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "extend-no-exp-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Extend the license (adding an expiration)
    let app = build_router(state);
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/extend", license_id),
        Some(json!({
            "new_expires_at": "2026-12-31"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert!(body["previous_expires_at"].is_null());
    assert!(body["new_expires_at"]
        .as_str()
        .unwrap()
        .contains("2026-12-31"));
}

#[tokio::test]
async fn extend_revoked_license() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create and revoke a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "extend-revoked-org",
            "features": ["test"],
            "expires_at": "2025-06-30"
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    let app = build_router(state.clone());
    let _ = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/revoke", license_id),
        Some(json!({ "grace_period_days": 0 })),
    )
    .await;

    // Extend the revoked license (should work - allows updating expiry even when revoked)
    let app = build_router(state);
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/extend", license_id),
        Some(json!({
            "new_expires_at": "2027-12-31"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn extend_license_not_found() {
    let state = setup_test_app().await;
    let app = build_router(state);

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/licenses/nonexistent-id/extend",
        Some(json!({
            "new_expires_at": "2027-12-31"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not found"));
}

#[tokio::test]
async fn extend_license_invalid_date() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "extend-invalid-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Try to extend with invalid date format
    let app = build_router(state);
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/extend", license_id),
        Some(json!({
            "new_expires_at": "not-a-date"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("datetime"));
}

// ============================================================================
// Update Usage Tests
// ============================================================================

#[tokio::test]
async fn update_usage_success() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "usage-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Update usage
    let app = build_router(state);
    let (status, body) = json_request(
        app,
        "PATCH",
        &format!("/api/v1/licenses/{}/usage", license_id),
        Some(json!({
            "bandwidth_used_bytes": 500_000_000,
            "bandwidth_limit_bytes": 1_000_000_000
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["bandwidth_used_bytes"], 500_000_000_u64);
    assert_eq!(body["bandwidth_limit_bytes"], 1_000_000_000_u64);
    assert_eq!(body["quota_exceeded"], false);
    assert_eq!(body["usage_percentage"], 50.0);
}

#[tokio::test]
async fn update_usage_quota_exceeded() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "usage-exceeded-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Update usage to exceed limit
    let app = build_router(state);
    let (status, body) = json_request(
        app,
        "PATCH",
        &format!("/api/v1/licenses/{}/usage", license_id),
        Some(json!({
            "bandwidth_used_bytes": 1_500_000_000_u64,
            "bandwidth_limit_bytes": 1_000_000_000_u64
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["quota_exceeded"], true);
    assert_eq!(body["usage_percentage"], 150.0);
}

#[tokio::test]
async fn update_usage_reset() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "usage-reset-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Reset usage
    let app = build_router(state);
    let (status, body) = json_request(
        app,
        "PATCH",
        &format!("/api/v1/licenses/{}/usage", license_id),
        Some(json!({
            "reset": true,
            "bandwidth_limit_bytes": 1_000_000_000_u64
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["bandwidth_used_bytes"], 0);
    assert_eq!(body["quota_exceeded"], false);
}

#[tokio::test]
async fn update_usage_no_limit() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "usage-no-limit-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Update usage without limit (unlimited)
    let app = build_router(state);
    let (status, body) = json_request(
        app,
        "PATCH",
        &format!("/api/v1/licenses/{}/usage", license_id),
        Some(json!({
            "bandwidth_used_bytes": 999_999_999_999_u64
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["quota_exceeded"], false);
    assert!(body["usage_percentage"].is_null());
    assert!(body["bandwidth_limit_bytes"].is_null());
}

#[tokio::test]
async fn update_usage_license_not_found() {
    let state = setup_test_app().await;
    let app = build_router(state);

    let (status, body) = json_request(
        app,
        "PATCH",
        "/api/v1/licenses/nonexistent-id/usage",
        Some(json!({
            "bandwidth_used_bytes": 1000
        })),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not found"));
}

// ============================================================================
// Blacklist License Tests
// ============================================================================

#[tokio::test]
async fn blacklist_license_success() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "blacklist-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Blacklist the license
    let app = build_router(state.clone());
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/blacklist", license_id),
        Some(json!({
            "reason": "Terms of service violation - abuse detected",
            "message": "This license has been permanently banned"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["status"], "revoked");
    assert_eq!(body["message"], "License has been blacklisted");
    assert!(!body["blacklisted_at"].as_str().unwrap().is_empty());

    // Verify the license is blacklisted and revoked
    let app = build_router(state);
    let (_, get_body) = json_request(
        app,
        "GET",
        &format!("/api/v1/licenses/{}", license_id),
        None,
    )
    .await;

    assert_eq!(get_body["status"], "revoked");
}

#[tokio::test]
async fn blacklist_license_clears_hardware_binding() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "blacklist-bound-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();
    let license_key = create_body["license_key"].as_str().unwrap();

    // Bind the license to hardware
    let app = build_router(state.clone());
    let _ = json_request(
        app,
        "POST",
        "/api/v1/client/bind",
        Some(json!({
            "license_key": license_key,
            "hardware_id": "test-hw-123",
            "device_name": "Test Device"
        })),
    )
    .await;

    // Verify it's bound
    let app = build_router(state.clone());
    let (_, get_body) = json_request(
        app,
        "GET",
        &format!("/api/v1/licenses/{}", license_id),
        None,
    )
    .await;
    assert_eq!(get_body["is_bound"], true);

    // Blacklist the license
    let app = build_router(state.clone());
    let (status, _) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/blacklist", license_id),
        Some(json!({
            "reason": "Fraud detected"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);

    // Verify hardware binding was cleared
    let app = build_router(state);
    let (_, get_body) = json_request(
        app,
        "GET",
        &format!("/api/v1/licenses/{}", license_id),
        None,
    )
    .await;

    assert_eq!(get_body["is_bound"], false);
    assert!(get_body["hardware_id"].is_null());
    assert!(get_body["device_name"].is_null());
}

#[tokio::test]
async fn blacklist_license_not_found() {
    let state = setup_test_app().await;
    let app = build_router(state);

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/licenses/nonexistent-id/blacklist",
        Some(json!({
            "reason": "Test reason"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not found"));
}

#[tokio::test]
async fn blacklist_license_already_blacklisted() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "double-blacklist-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Blacklist it
    let app = build_router(state.clone());
    let _ = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/blacklist", license_id),
        Some(json!({
            "reason": "First blacklist"
        })),
    )
    .await;

    // Try to blacklist again
    let app = build_router(state);
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/blacklist", license_id),
        Some(json!({
            "reason": "Second blacklist attempt"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("already blacklisted"));
}

#[tokio::test]
async fn blacklist_license_empty_reason() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "blacklist-empty-reason-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Try to blacklist with empty reason
    let app = build_router(state);
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/blacklist", license_id),
        Some(json!({
            "reason": "   "
        })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("reason is required"));
}

#[tokio::test]
async fn reinstate_blacklisted_license_fails() {
    let state = setup_test_app().await;
    let app = build_router(state.clone());

    // Create a license
    let (_, create_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/licenses",
        Some(json!({
            "org_id": "reinstate-blacklist-org",
            "features": ["test"]
        })),
    )
    .await;

    let license_id = create_body["license_id"].as_str().unwrap();

    // Blacklist it
    let app = build_router(state.clone());
    let _ = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/blacklist", license_id),
        Some(json!({
            "reason": "Permanent ban"
        })),
    )
    .await;

    // Try to reinstate the blacklisted license
    let app = build_router(state);
    let (status, body) = json_request(
        app,
        "POST",
        &format!("/api/v1/licenses/{}/reinstate", license_id),
        Some(json!({
            "reason": "Trying to unban"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Cannot reinstate a blacklisted license"));
}
