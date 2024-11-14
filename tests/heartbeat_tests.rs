use reqwest::Client;
use serde_json::json;
use std::time::Duration;

#[tokio::test]
async fn test_activate_and_send_heartbeat() {
    let client = Client::new();
    let server_url = "http://127.0.0.1:8080";

    // Step 1: Activate a license
    let license_id = "LICENSE-12345";
    let client_id = "CLIENT-67890";

    let activate_response = client
        .post(format!("{}/activate", server_url))
        .json(&json!({
            "license_id": license_id,
            "client_id": client_id
        }))
        .send()
        .await
        .expect("Failed to send activate request");

    assert!(
        activate_response.status().is_success(),
        "Failed to activate license"
    );

    let activate_result: serde_json::Value = activate_response.json().await.expect("Invalid response");
    assert_eq!(activate_result["success"], true, "License activation was not successful");

    // Step 2: Send a heartbeat
    let heartbeat_response = client
        .post(format!("{}/heartbeat", server_url))
        .json(&json!({
            "license_id": license_id,
            "client_id": client_id
        }))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to send heartbeat request");

    assert!(
        heartbeat_response.status().is_success(),
        "Failed to send heartbeat"
    );

    let heartbeat_result: serde_json::Value = heartbeat_response.json().await.expect("Invalid response");
    assert_eq!(heartbeat_result["success"], true, "Heartbeat was not successful");
}

#[tokio::test]
async fn test_invalid_heartbeat() {
    let client = Client::new();
    let server_url = "http://127.0.0.1:8080";

    // Step 1: Send a heartbeat without activating license
    let license_id = "INVALID-LICENSE";
    let client_id = "INVALID-CLIENT";

    let heartbeat_response = client
        .post(format!("{}/heartbeat", server_url))
        .json(&json!({
            "license_id": license_id,
            "client_id": client_id
        }))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to send heartbeat request");

    assert!(
        heartbeat_response.status().is_success(),
        "Failed to send heartbeat"
    );

    let heartbeat_result: serde_json::Value = heartbeat_response.json().await.expect("Invalid response");
    assert_eq!(heartbeat_result["success"], false, "Invalid license should not succeed in heartbeat");
}
