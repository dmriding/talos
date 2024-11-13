use reqwest::Client;
use crate::client::client::License;
use crate::errors::LicenseError;

/// Sends a heartbeat to the server to validate that the license is still active.
pub async fn send_heartbeat(
    license: &License,
    client_id: &str,
    rotating_key: &str,
) -> Result<bool, LicenseError> {
    let server_url = license.server_url.clone();
    let client = Client::new();
    
    let response = client.post(format!("{}/heartbeat", server_url))
        .json(&serde_json::json!({
            "client_id": client_id,
            "rotating_key": rotating_key
        }))
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => Ok(true),
        _ => Err(LicenseError::ServerError("Heartbeat failed".to_string())),
    }
}
