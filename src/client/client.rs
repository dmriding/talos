use crate::config::get_server_url;
use crate::client::heartbeat::send_heartbeat;
use crate::errors::LicenseError;
use crate::hardware::get_hardware_id;
use serde::{Deserialize, Serialize};
use reqwest::Client;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct License {
    pub license_id: String,
    pub client_id: String,
    pub expiry_date: String,
    pub features: Vec<String>,
    pub server_url: String,
    pub signature: String,
    pub is_active: bool, // Field to track activation status
}

impl License {
    /// Function to validate the license using heartbeat and hardware binding
    pub async fn validate(&self) -> Result<bool, LicenseError> {
        let current_hardware_id = get_hardware_id();
        
        // Check if the license is bound to the current hardware
        if self.client_id != current_hardware_id {
            return Err(LicenseError::InvalidLicense("Hardware mismatch.".to_string()));
        }

        // Check if the license is active
        if !self.is_active {
            return Err(LicenseError::InvalidLicense("License is inactive.".to_string()));
        }

        // Send a heartbeat to the server to confirm the license is still valid
        let rotating_key = "example_rotating_key"; // Placeholder
        send_heartbeat(self, &rotating_key).await
    }

    /// Function to activate the license
    pub async fn activate(&mut self) -> Result<(), LicenseError> {
        let server_url = get_server_url(self);
        let client_id = get_hardware_id();
        
        let response = Client::new()
            .post(format!("{}/activate", server_url))
            .json(&serde_json::json!({
                "license_id": self.license_id,
                "client_id": client_id
            }))
            .send()
            .await?;

        if response.status().is_success() {
            self.is_active = true;
            self.client_id = client_id; // Bind the license to this hardware
            println!("License activated successfully.");
            Ok(())
        } else {
            Err(LicenseError::ServerError("Activation failed".to_string()))
        }
    }

    /// Function to deactivate the license
    pub async fn deactivate(&mut self) -> Result<(), LicenseError> {
        let server_url = get_server_url(self);
        let client_id = get_hardware_id();
        
        let response = Client::new()
            .post(format!("{}/deactivate", server_url))
            .json(&serde_json::json!({
                "license_id": self.license_id,
                "client_id": client_id
            }))
            .send()
            .await?;

        if response.status().is_success() {
            self.is_active = false;
            println!("License deactivated successfully.");
            Ok(())
        } else {
            Err(LicenseError::ServerError("Deactivation failed".to_string()))
        }
    }
}
