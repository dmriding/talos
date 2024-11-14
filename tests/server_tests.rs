use std::sync::Arc;
use axum::{Router, routing::post, extract::State};
use axum::http::{Request, StatusCode};
use tokio::sync::Mutex;
use tower::ServiceExt;
use reqwest::Client;
use serde_json::json;
use talos::server::{database::Database, handlers::{activate_license_handler, validate_license_handler, deactivate_license_handler}};
use talos::server::handlers::LicenseRequest;

#[tokio::test]
async fn test_server_endpoints() {
    // Initialize the in-memory SQLite database
    let db = Database::new().await;
    let db_state = Arc::new(db);

    // Create the router
    let app = Router::new()
        .route("/activate", post(activate_license_handler))
        .route("/validate", post(validate_license_handler))
        .route("/deactivate", post(deactivate_license_handler))
        .with_state(db_state.clone());

    // Create a test client
    let client = Client::new();

    // --- Test Activate Endpoint ---
    let activate_payload = json!({
        "license_id": "LICENSE-123",
        "client_id": "CLIENT-456"
    });

    let activate_response = client
        .post("http://127.0.0.1:8080/activate")
        .json(&activate_payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(activate_response.status(), StatusCode::OK);

    // --- Test Validate Endpoint ---
    let validate_payload = json!({
        "license_id": "LICENSE-123",
        "client_id": "CLIENT-456"
    });

    let validate_response = client
        .post("http://127.0.0.1:8080/validate")
        .json(&validate_payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(validate_response.status(), StatusCode::OK);
    let validate_result: serde_json::Value = validate_response.json().await.unwrap();
    assert!(validate_result["success"].as_bool().unwrap());

    // --- Test Deactivate Endpoint ---
    let deactivate_payload = json!({
        "license_id": "LICENSE-123",
        "client_id": "CLIENT-456"
    });

    let deactivate_response = client
        .post("http://127.0.0.1:8080/deactivate")
        .json(&deactivate_payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(deactivate_response.status(), StatusCode::OK);
    let deactivate_result: serde_json::Value = deactivate_response.json().await.unwrap();
    assert!(deactivate_result["success"].as_bool().unwrap());
}
