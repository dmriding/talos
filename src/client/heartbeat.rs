use reqwest::Client;
use crate::client::client::License;
use crate::errors::LicenseError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Struct for the heartbeat request payload
#[derive(Debug, Serialize)]
struct HeartbeatRequest {
    client_id: String,
    license_id: String,
    rotating_key: String,
}

/// Struct for the server response
#[derive(Debug, Deserialize)]
struct HeartbeatResponse {
    success: bool,
}

/// Sends a heartbeat to the server to validate that the license is still active.
pub async fn send_heartbeat(
    license: &License,
    rotating_key: &str,
) -> Result<bool, LicenseError> {
    let server_url = license.server_url.clone();
    let client = Client::new();

    // Prepare the heartbeat payload
    let payload = HeartbeatRequest {
        client_id: license.client_id.clone(),
        license_id: license.license_id.clone(),
        rotating_key: rotating_key.to_string(),
    };

    // Send the heartbeat request to the server
    let response = client
        .post(format!("{}/heartbeat", server_url))
        .json(&payload)
        .timeout(Duration::from_secs(10)) // Set a timeout to prevent hanging
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            let heartbeat_response: HeartbeatResponse = resp.json().await.map_err(|_| {
                LicenseError::ServerError("Failed to parse server response".to_string())
            })?;
            if heartbeat_response.success {
                Ok(true)
            } else {
                Err(LicenseError::InvalidLicense("License validation failed".to_string()))
            }            
        }
        Ok(resp) => {
            Err(LicenseError::ServerError(format!(
                "Unexpected server response: {}",
                resp.status()
            )))
        }
        Err(err) => Err(LicenseError::ServerError(format!(
            "Failed to send heartbeat: {}",
            err
        ))),
    }
}
