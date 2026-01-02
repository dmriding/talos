use crate::client::encrypted_storage::{
    clear_license_from_disk, load_license_from_disk, save_license_to_disk,
};
use crate::client::heartbeat::send_heartbeat;
use crate::errors::{LicenseError, LicenseResult};
use crate::hardware::get_hardware_id;

use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Core license representation stored locally and exchanged with the server.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct License {
    /// Server-side license identifier.
    pub license_id: String,
    /// Hardware-bound client identifier (derived from hardware fingerprint).
    pub client_id: String,
    /// ISO8601 expiry date or whatever format your server uses.
    pub expiry_date: String,
    /// Enabled feature flags for this license.
    pub features: Vec<String>,
    /// Base URL of the licensing server.
    pub server_url: String,
    /// Server-side signature over the license payload.
    pub signature: String,
    /// Local activation flag (client-side view).
    pub is_active: bool,
}

/// Request payload for license operations (/activate, /validate, /deactivate).
#[derive(Debug, Serialize)]
struct LicenseRequest {
    pub license_id: String,
    pub client_id: String,
}

/// Response payload for license operations.
#[derive(Debug, Deserialize)]
struct LicenseResponse {
    pub success: bool,
}

impl License {
    /// Validate the license against:
    /// - local hardware binding,
    /// - local active flag,
    /// - server's `/validate` endpoint.
    ///
    /// Returns:
    /// - `Ok(true)` if valid.
    /// - `Err(InvalidLicense)` if invalid/mismatch/inactive.
    /// - `Err(NetworkError | ServerError)` on transport/protocol errors.
    pub async fn validate(&self) -> LicenseResult<bool> {
        let current_hardware_id = get_hardware_id();

        // Hardware binding check (client-side).
        if self.client_id != current_hardware_id {
            return Err(LicenseError::InvalidLicense(
                "Hardware mismatch for this license.".to_string(),
            ));
        }

        // Local active flag check.
        if !self.is_active {
            return Err(LicenseError::InvalidLicense(
                "License is marked inactive locally.".to_string(),
            ));
        }

        // Use the server URL embedded in the license.
        let server_url = &self.server_url;

        let payload = LicenseRequest {
            license_id: self.license_id.clone(),
            client_id: self.client_id.clone(),
        };

        let resp = Client::new()
            .post(format!("{}/validate", server_url))
            .json(&payload)
            .send()
            .await?; // → LicenseError::NetworkError

        if !resp.status().is_success() {
            return Err(LicenseError::ServerError(format!(
                "Validation failed with HTTP status {}",
                resp.status()
            )));
        }

        let body: LicenseResponse = resp.json().await.map_err(|e| {
            LicenseError::ServerError(format!("Failed to parse validate response: {e}"))
        })?;

        if body.success {
            Ok(true)
        } else {
            Err(LicenseError::InvalidLicense(
                "Server reported license as invalid.".to_string(),
            ))
        }
    }

    /// Activate the license on the server and persist it encrypted on disk,
    /// bound to the current hardware ID.
    pub async fn activate(&mut self) -> LicenseResult<()> {
        // Use the server URL embedded in the license.
        let server_url = &self.server_url;
        let client_id = get_hardware_id();

        let payload = LicenseRequest {
            license_id: self.license_id.clone(),
            client_id: client_id.clone(),
        };

        let resp = Client::new()
            .post(format!("{}/activate", server_url))
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(LicenseError::ServerError(format!(
                "Activation failed with HTTP status {}",
                resp.status()
            )));
        }

        let body: LicenseResponse = resp.json().await.map_err(|e| {
            LicenseError::ServerError(format!("Failed to parse activation response: {e}"))
        })?;

        if !body.success {
            return Err(LicenseError::InvalidLicense(
                "Activation failed on server.".to_string(),
            ));
        }

        // Bind locally and persist encrypted.
        self.is_active = true;
        self.client_id = client_id;

        save_license_to_disk(self).await?;

        println!("License activated successfully.");
        Ok(())
    }

    /// Deactivate the license on the server and clear local encrypted storage.
    pub async fn deactivate(&mut self) -> LicenseResult<()> {
        // Use the server URL embedded in the license.
        let server_url = &self.server_url;
        let client_id = get_hardware_id();

        let payload = LicenseRequest {
            license_id: self.license_id.clone(),
            client_id,
        };

        let resp = Client::new()
            .post(format!("{}/deactivate", server_url))
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(LicenseError::ServerError(format!(
                "Deactivation failed with HTTP status {}",
                resp.status()
            )));
        }

        let body: LicenseResponse = resp.json().await.map_err(|e| {
            LicenseError::ServerError(format!("Failed to parse deactivation response: {e}"))
        })?;

        if !body.success {
            return Err(LicenseError::InvalidLicense(
                "Deactivation failed on server.".to_string(),
            ));
        }

        self.is_active = false;
        clear_license_from_disk().await?;

        println!("License deactivated successfully.");
        Ok(())
    }

    /// Explicit heartbeat wrapper on the client.
    ///
    /// - Enforces hardware binding before hitting the server.
    /// - Calls `/heartbeat` and returns `true` iff the server updated a row.
    pub async fn heartbeat(&self) -> LicenseResult<bool> {
        let current_hardware_id = get_hardware_id();

        if self.client_id != current_hardware_id {
            return Err(LicenseError::InvalidLicense(
                "Hardware mismatch for heartbeat.".to_string(),
            ));
        }

        let ok = send_heartbeat(self).await?;
        Ok(ok)
    }

    /// Load the license from encrypted local storage.
    pub async fn load_from_disk() -> LicenseResult<Self> {
        load_license_from_disk().await
    }

    /// Save the license to encrypted local storage.
    pub async fn save_to_disk(&self) -> LicenseResult<()> {
        save_license_to_disk(self).await
    }

    /// Clear local encrypted license storage (no-op if file is absent).
    pub async fn clear_local_cache(&self) -> LicenseResult<()> {
        clear_license_from_disk().await
    }
}
