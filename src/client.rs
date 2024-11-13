use crate::heartbeat::send_heartbeat;
use crate::hardware::get_hardware_id;
use crate::errors::LicenseError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct License {
    pub license_id: String,
    pub client_id: String,
    pub expiry_date: String,
    pub features: Vec<String>,
    pub server_url: String,
    pub signature: String,
}

impl License {
    pub async fn validate(&self) -> Result<bool, LicenseError> {
        let client_id = get_hardware_id();
        let rotating_key = "example_rotating_key"; // Placeholder for actual key generation
        send_heartbeat(self, &client_id, &rotating_key).await
    }
}
