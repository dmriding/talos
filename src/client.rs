use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct License {
    /// The unique ID of the license
    pub license_id: String,
    /// The unique ID of the client (hardware-based)
    pub client_id: String,
    /// The expiry date of the license in ISO format (e.g., "2025-12-31")
    pub expiry_date: String,
    /// List of features enabled by this license
    pub features: Vec<String>,
    /// The URL of the licensing server to communicate with
    pub server_url: String,
    /// The digital signature to validate the license
    pub signature: String,
}

impl License {
    /// Function to create a new instance of the License struct
    pub fn new(
        license_id: &str,
        client_id: &str,
        expiry_date: &str,
        features: Vec<String>,
        server_url: &str,
        signature: &str,
    ) -> Self {
        Self {
            license_id: license_id.to_string(),
            client_id: client_id.to_string(),
            expiry_date: expiry_date.to_string(),
            features,
            server_url: server_url.to_string(),
            signature: signature.to_string(),
        }
    }
}
