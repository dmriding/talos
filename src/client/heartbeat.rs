use crate::client::client::License;
use crate::errors::{LicenseError, LicenseResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Struct for the heartbeat request payload sent to the server.
#[derive(Debug, Serialize)]
struct HeartbeatRequest {
    /// Server-side license identifier.
    license_id: String,
    /// Hardware-bound client identifier.
    client_id: String,
}

/// Struct for the heartbeat response returned by the server.
#[derive(Debug, Deserialize)]
struct HeartbeatResponse {
    /// Whether the heartbeat was accepted (row updated).
    success: bool,
}

/// Sends a heartbeat to the server to update `last_heartbeat`
/// for this license/client pair.
///
/// Returns:
/// - `Ok(true)` if `update_last_heartbeat` updated at least one row.
/// - `Ok(false)` if no matching license/client was found.
/// - `Err(NetworkError | ServerError)` on transport/protocol errors.
pub async fn send_heartbeat(license: &License) -> LicenseResult<bool> {
    let server_url = &license.server_url;
    let client = Client::new();

    let payload = HeartbeatRequest {
        license_id: license.license_id.clone(),
        client_id: license.client_id.clone(),
    };

    let resp = client
        .post(format!("{}/heartbeat", server_url))
        .json(&payload)
        .timeout(Duration::from_secs(10))
        .send()
        .await?; // → LicenseError::NetworkError via #[from] reqwest::Error

    if !resp.status().is_success() {
        return Err(LicenseError::ServerError(format!(
            "Unexpected server response: {}",
            resp.status()
        )));
    }

    let heartbeat_response: HeartbeatResponse = resp.json().await.map_err(|e| {
        LicenseError::ServerError(format!("Failed to parse heartbeat response: {e}"))
    })?;

    // Server semantics: success == `updated` flag from DB.
    Ok(heartbeat_response.success)
}
