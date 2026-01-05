//! Core license client for Talos.
//!
//! This module provides the main `License` struct for interacting with the
//! Talos license server. It supports both online and offline (air-gapped)
//! validation modes.
//!
//! ## Basic Usage (Online)
//!
//! ```rust,ignore
//! use talos::client::license::License;
//! use talos::errors::LicenseResult;
//!
//! #[tokio::main]
//! async fn main() -> LicenseResult<()> {
//!     // Create a new license client
//!     let mut license = License::new(
//!         "LIC-XXXX-XXXX-XXXX".to_string(),
//!         "https://license.example.com".to_string(),
//!     );
//!
//!     // Bind to this hardware
//!     let bind_result = license.bind(Some("My Workstation"), None).await?;
//!     println!("Features: {:?}", bind_result.features);
//!
//!     // Validate the license
//!     let validation = license.validate().await?;
//!     if validation.has_feature("feature_a") {
//!         println!("Feature A is enabled!");
//!     }
//!
//!     // Send heartbeat
//!     license.heartbeat().await?;
//!
//!     // Release when done
//!     license.release().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Air-Gapped Usage (Offline)
//!
//! ```rust,ignore
//! use talos::client::license::License;
//!
//! async fn check_license() -> Result<bool, Box<dyn std::error::Error>> {
//!     let mut license = License::load_from_disk().await?;
//!
//!     // Try online first, fall back to cached validation
//!     match license.validate_with_fallback().await {
//!         Ok(result) => {
//!             if let Some(warning) = &result.warning {
//!                 eprintln!("Warning: {}", warning);
//!             }
//!             Ok(true)
//!         }
//!         Err(e) => {
//!             eprintln!("License validation failed: {}", e);
//!             Ok(false)
//!         }
//!     }
//! }
//! ```

use crate::client::cache::{
    clear_cache_from_disk, load_cache_from_disk, save_cache_to_disk, CachedValidation,
};
use crate::client::encrypted_storage::{
    clear_license_from_disk, load_license_from_disk, save_license_to_disk,
};
use crate::client::errors::{ClientApiError, ClientErrorCode, ServerErrorResponse};
use crate::client::responses::{
    BindResult, FeatureResult, HeartbeatResult, ServerBindResponse, ServerFeatureResponse,
    ServerHeartbeatResponse, ServerReleaseResponse, ServerValidateResponse, ValidationResult,
};
use crate::errors::{LicenseError, LicenseResult};
use crate::hardware::get_hardware_id;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// HTTP client timeout for license server requests.
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Core license representation.
///
/// This struct holds the credentials needed to interact with the license server
/// and optionally cached validation state for offline use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    /// Human-readable license key (e.g., "LIC-XXXX-XXXX-XXXX")
    pub license_key: String,

    /// Base URL of the licensing server
    pub server_url: String,

    /// Hardware ID this license is bound to (set after bind)
    #[serde(default)]
    pub hardware_id: String,

    /// Cached validation state for offline use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached: Option<CachedValidation>,

    // === Legacy fields for backwards compatibility ===
    // These are kept for deserializing old license files
    /// Legacy: Server-side license identifier (UUID)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub license_id: String,

    /// Legacy: Alias for hardware_id
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub client_id: String,

    /// Legacy: Expiry date string
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub expiry_date: String,

    /// Legacy: Features list
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,

    /// Legacy: Server signature
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub signature: String,

    /// Legacy: Active flag
    #[serde(default)]
    pub is_active: bool,
}

// === Request Types ===

#[derive(Debug, Serialize)]
struct BindRequest {
    license_key: String,
    hardware_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_info: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReleaseRequest {
    license_key: String,
    hardware_id: String,
}

#[derive(Debug, Serialize)]
struct ValidateRequest {
    license_key: String,
    hardware_id: String,
}

#[derive(Debug, Serialize)]
struct HeartbeatRequest {
    license_key: String,
    hardware_id: String,
}

#[derive(Debug, Serialize)]
struct FeatureRequest {
    license_key: String,
    hardware_id: String,
    feature: String,
}

// === Legacy Request Types ===

#[derive(Debug, Serialize)]
struct LegacyLicenseRequest {
    license_id: String,
    client_id: String,
}

#[derive(Debug, Deserialize)]
struct LegacyLicenseResponse {
    success: bool,
}

impl License {
    /// Create a new license client.
    ///
    /// The license is not bound to any hardware until `bind()` is called.
    pub fn new(license_key: String, server_url: String) -> Self {
        Self {
            license_key,
            server_url,
            hardware_id: String::new(),
            cached: None,
            // Legacy fields
            license_id: String::new(),
            client_id: String::new(),
            expiry_date: String::new(),
            features: Vec::new(),
            signature: String::new(),
            is_active: false,
        }
    }

    /// Create an HTTP client with standard timeout.
    fn http_client() -> Client {
        Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| Client::new())
    }

    /// Parse an error response from the server.
    async fn parse_error_response(resp: reqwest::Response) -> LicenseError {
        let status = resp.status();

        // Try to parse as structured error
        match resp.json::<ServerErrorResponse>().await {
            Ok(err_resp) => LicenseError::ClientApiError(err_resp.into()),
            Err(_) => LicenseError::ServerError(format!("Request failed with status {}", status)),
        }
    }

    // =========================================================================
    // New API Methods (v1)
    // =========================================================================

    /// Bind this license to the current hardware.
    ///
    /// This registers the license key with the server and associates it with
    /// this device's hardware fingerprint.
    ///
    /// # Arguments
    ///
    /// * `device_name` - Optional human-readable name for this device
    /// * `device_info` - Optional device information (OS, version, etc.)
    ///
    /// # Returns
    ///
    /// On success, returns `BindResult` with license details including features
    /// and tier information.
    pub async fn bind(
        &mut self,
        device_name: Option<&str>,
        device_info: Option<&str>,
    ) -> LicenseResult<BindResult> {
        let hardware_id = get_hardware_id();

        let request = BindRequest {
            license_key: self.license_key.clone(),
            hardware_id: hardware_id.clone(),
            device_name: device_name.map(|s| s.to_string()),
            device_info: device_info.map(|s| s.to_string()),
        };

        let resp = Self::http_client()
            .post(format!("{}/api/v1/client/bind", self.server_url))
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::parse_error_response(resp).await);
        }

        let server_resp: ServerBindResponse = resp.json().await.map_err(|e| {
            LicenseError::ServerError(format!("Failed to parse bind response: {e}"))
        })?;

        // Update local state
        self.hardware_id = hardware_id.clone();
        self.is_active = true;

        // Update legacy fields for backwards compatibility
        self.license_id = server_resp.license_id.clone();
        self.client_id = hardware_id.clone();
        self.features = server_resp.features.clone();
        self.expiry_date = server_resp.expires_at.clone().unwrap_or_default();

        // Save to disk
        self.save_to_disk().await?;

        Ok(server_resp.into())
    }

    /// Release this license from the current hardware.
    ///
    /// This unbinds the license from this device, allowing it to be bound
    /// to a different device.
    pub async fn release(&mut self) -> LicenseResult<()> {
        if self.hardware_id.is_empty() {
            return Err(LicenseError::InvalidLicense(
                "License is not bound to any hardware.".to_string(),
            ));
        }

        let request = ReleaseRequest {
            license_key: self.license_key.clone(),
            hardware_id: self.hardware_id.clone(),
        };

        let resp = Self::http_client()
            .post(format!("{}/api/v1/client/release", self.server_url))
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::parse_error_response(resp).await);
        }

        let _: ServerReleaseResponse = resp.json().await.map_err(|e| {
            LicenseError::ServerError(format!("Failed to parse release response: {e}"))
        })?;

        // Clear local state
        self.hardware_id.clear();
        self.is_active = false;
        self.client_id.clear();
        self.cached = None;

        // Clear from disk
        clear_license_from_disk().await?;
        clear_cache_from_disk().await?;

        Ok(())
    }

    /// Validate the license online with the server.
    ///
    /// This calls the server to validate the license and updates the local
    /// cache with the response (for offline use).
    ///
    /// # Returns
    ///
    /// On success, returns `ValidationResult` with current license state.
    /// If the license has a grace period warning, it will be included.
    pub async fn validate(&mut self) -> LicenseResult<ValidationResult> {
        self.ensure_bound()?;

        let request = ValidateRequest {
            license_key: self.license_key.clone(),
            hardware_id: self.hardware_id.clone(),
        };

        let resp = Self::http_client()
            .post(format!("{}/api/v1/client/validate", self.server_url))
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::parse_error_response(resp).await);
        }

        let server_resp: ServerValidateResponse = resp.json().await.map_err(|e| {
            LicenseError::ServerError(format!("Failed to parse validate response: {e}"))
        })?;

        let result: ValidationResult = server_resp.into();

        // Update cache for offline use
        self.cached = Some(CachedValidation::new(
            self.license_key.clone(),
            self.hardware_id.clone(),
            result.features.clone(),
            result.tier.clone(),
            result.expires_at.clone(),
            result.grace_period_ends_at.clone(),
        ));

        // Save updated cache
        if let Some(ref cache) = self.cached {
            let _ = save_cache_to_disk(cache).await;
        }
        self.save_to_disk().await?;

        Ok(result)
    }

    /// Validate the license using cached state (offline mode).
    ///
    /// This checks the locally cached validation data without contacting
    /// the server. Use this for air-gapped systems during the grace period.
    ///
    /// # Returns
    ///
    /// Returns `Ok(ValidationResult)` if:
    /// - There is a valid cache for this license/hardware
    /// - The grace period has not expired
    /// - The license itself has not expired
    ///
    /// Returns `Err` if:
    /// - No cache exists
    /// - Grace period has expired (must go online)
    /// - License has expired
    pub fn validate_offline(&self) -> LicenseResult<ValidationResult> {
        // Try to get cache from memory first, then disk
        let cache = match &self.cached {
            Some(c) => c.clone(),
            None => {
                // Try loading from disk synchronously is tricky, so we'll
                // require the cache to be in memory for offline validation.
                // Users should call load_from_disk() first.
                return Err(LicenseError::InvalidLicense(
                    "No cached validation available. Call validate() while online first."
                        .to_string(),
                ));
            }
        };

        // Verify hardware binding
        if !cache.matches_hardware() {
            return Err(LicenseError::InvalidLicense(
                "Cached validation does not match current hardware.".to_string(),
            ));
        }

        // Check if license key matches
        if cache.license_key != self.license_key {
            return Err(LicenseError::InvalidLicense(
                "Cached validation is for a different license.".to_string(),
            ));
        }

        // Check if license has expired
        if cache.is_license_expired() {
            return Err(LicenseError::ClientApiError(ClientApiError::new(
                ClientErrorCode::LicenseExpired,
                "License has expired.",
            )));
        }

        // Check if grace period is still valid
        if !cache.is_valid_for_offline() {
            return Err(LicenseError::ClientApiError(
                ClientApiError::grace_period_expired(),
            ));
        }

        // Build result from cache
        let warning = cache.grace_period_ends_at.as_ref().map(|ends_at| {
            format!(
                "Offline mode - license must be validated online before {}",
                ends_at
            )
        });

        Ok(ValidationResult {
            features: cache.features.clone(),
            tier: cache.tier.clone(),
            expires_at: cache.expires_at.clone(),
            grace_period_ends_at: cache.grace_period_ends_at.clone(),
            warning,
        })
    }

    /// Validate with automatic fallback to offline mode.
    ///
    /// This tries to validate online first. If the network request fails,
    /// it falls back to offline validation using the cached state.
    ///
    /// This is the recommended method for air-gapped systems.
    pub async fn validate_with_fallback(&mut self) -> LicenseResult<ValidationResult> {
        match self.validate().await {
            Ok(result) => Ok(result),
            Err(LicenseError::NetworkError(_)) => {
                // Network failed, try offline validation
                self.validate_offline()
            }
            Err(e) => Err(e),
        }
    }

    /// Validate a specific feature for this license.
    ///
    /// This always calls the server (authoritative check).
    pub async fn validate_feature(&self, feature: &str) -> LicenseResult<FeatureResult> {
        self.ensure_bound()?;

        let request = FeatureRequest {
            license_key: self.license_key.clone(),
            hardware_id: self.hardware_id.clone(),
            feature: feature.to_string(),
        };

        let resp = Self::http_client()
            .post(format!(
                "{}/api/v1/client/validate-feature",
                self.server_url
            ))
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::parse_error_response(resp).await);
        }

        let server_resp: ServerFeatureResponse = resp.json().await.map_err(|e| {
            LicenseError::ServerError(format!("Failed to parse feature response: {e}"))
        })?;

        Ok(server_resp.into())
    }

    /// Send a heartbeat to the server.
    ///
    /// This updates the server's `last_seen_at` timestamp for this license
    /// and refreshes the grace period for air-gapped systems.
    pub async fn heartbeat(&mut self) -> LicenseResult<HeartbeatResult> {
        self.ensure_bound()?;

        let request = HeartbeatRequest {
            license_key: self.license_key.clone(),
            hardware_id: self.hardware_id.clone(),
        };

        let resp = Self::http_client()
            .post(format!("{}/api/v1/client/heartbeat", self.server_url))
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::parse_error_response(resp).await);
        }

        let server_resp: ServerHeartbeatResponse = resp.json().await.map_err(|e| {
            LicenseError::ServerError(format!("Failed to parse heartbeat response: {e}"))
        })?;

        let result: HeartbeatResult = server_resp.into();

        // If server returned a new grace period, update cache
        if let Some(ref new_grace) = result.grace_period_ends_at {
            if let Some(ref mut cache) = self.cached {
                cache.grace_period_ends_at = Some(new_grace.clone());
                let _ = save_cache_to_disk(cache).await;
            }
        }

        Ok(result)
    }

    // =========================================================================
    // Legacy API Methods (deprecated)
    // =========================================================================

    /// Activate the license on the server.
    ///
    /// **Deprecated:** Use `bind()` instead.
    #[deprecated(since = "0.2.0", note = "Use bind() instead")]
    pub async fn activate(&mut self) -> LicenseResult<()> {
        let server_url = &self.server_url;
        let client_id = get_hardware_id();

        let payload = LegacyLicenseRequest {
            license_id: self.license_id.clone(),
            client_id: client_id.clone(),
        };

        let resp = Self::http_client()
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

        let body: LegacyLicenseResponse = resp.json().await.map_err(|e| {
            LicenseError::ServerError(format!("Failed to parse activation response: {e}"))
        })?;

        if !body.success {
            return Err(LicenseError::InvalidLicense(
                "Activation failed on server.".to_string(),
            ));
        }

        self.is_active = true;
        self.client_id = client_id.clone();
        self.hardware_id = client_id;

        save_license_to_disk(self).await?;

        Ok(())
    }

    /// Deactivate the license on the server.
    ///
    /// **Deprecated:** Use `release()` instead.
    #[deprecated(since = "0.2.0", note = "Use release() instead")]
    pub async fn deactivate(&mut self) -> LicenseResult<()> {
        let server_url = &self.server_url;
        let client_id = get_hardware_id();

        let payload = LegacyLicenseRequest {
            license_id: self.license_id.clone(),
            client_id,
        };

        let resp = Self::http_client()
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

        let body: LegacyLicenseResponse = resp.json().await.map_err(|e| {
            LicenseError::ServerError(format!("Failed to parse deactivation response: {e}"))
        })?;

        if !body.success {
            return Err(LicenseError::InvalidLicense(
                "Deactivation failed on server.".to_string(),
            ));
        }

        self.is_active = false;
        clear_license_from_disk().await?;

        Ok(())
    }

    /// Send a heartbeat using the legacy API.
    ///
    /// **Deprecated:** Use `heartbeat()` instead (which returns `HeartbeatResult`).
    #[deprecated(since = "0.2.0", note = "Use heartbeat() instead")]
    pub async fn legacy_heartbeat(&self) -> LicenseResult<bool> {
        let current_hardware_id = get_hardware_id();

        if self.client_id != current_hardware_id {
            return Err(LicenseError::InvalidLicense(
                "Hardware mismatch for heartbeat.".to_string(),
            ));
        }

        // Use the old heartbeat module
        crate::client::heartbeat::send_heartbeat(self).await
    }

    // =========================================================================
    // Storage Methods
    // =========================================================================

    /// Load the license from encrypted local storage.
    pub async fn load_from_disk() -> LicenseResult<Self> {
        let mut license = load_license_from_disk().await?;

        // Also try to load cached validation
        if let Ok(cache) = load_cache_from_disk().await {
            license.cached = Some(cache);
        }

        Ok(license)
    }

    /// Save the license to encrypted local storage.
    pub async fn save_to_disk(&self) -> LicenseResult<()> {
        save_license_to_disk(self).await
    }

    /// Clear all local storage (license and cache).
    pub async fn clear_local_storage() -> LicenseResult<()> {
        clear_license_from_disk().await?;
        clear_cache_from_disk().await?;
        Ok(())
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Ensure the license is bound to hardware before making server requests.
    fn ensure_bound(&self) -> LicenseResult<()> {
        if self.hardware_id.is_empty() {
            return Err(LicenseError::InvalidLicense(
                "License is not bound. Call bind() first.".to_string(),
            ));
        }

        // Verify we're on the right hardware
        if self.hardware_id != get_hardware_id() {
            return Err(LicenseError::ClientApiError(ClientApiError::new(
                ClientErrorCode::HardwareMismatch,
                "License is bound to different hardware.",
            )));
        }

        Ok(())
    }

    /// Check if the license is currently bound to hardware.
    pub fn is_bound(&self) -> bool {
        !self.hardware_id.is_empty()
    }

    /// Get the license key.
    pub fn key(&self) -> &str {
        &self.license_key
    }

    /// Get the server URL.
    pub fn server(&self) -> &str {
        &self.server_url
    }

    /// Get the hardware ID this license is bound to.
    pub fn bound_hardware(&self) -> Option<&str> {
        if self.hardware_id.is_empty() {
            None
        } else {
            Some(&self.hardware_id)
        }
    }

    /// Get the cached validation state, if available.
    pub fn cached_validation(&self) -> Option<&CachedValidation> {
        self.cached.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_license_is_unbound() {
        let license = License::new(
            "TEST-XXXX-XXXX-XXXX".to_string(),
            "http://localhost:8080".to_string(),
        );

        assert!(!license.is_bound());
        assert!(license.bound_hardware().is_none());
        assert_eq!(license.key(), "TEST-XXXX-XXXX-XXXX");
        assert_eq!(license.server(), "http://localhost:8080");
    }

    #[test]
    fn ensure_bound_fails_when_unbound() {
        let license = License::new(
            "TEST-XXXX-XXXX-XXXX".to_string(),
            "http://localhost:8080".to_string(),
        );

        let result = license.ensure_bound();
        assert!(result.is_err());
    }

    #[test]
    fn validate_offline_requires_cache() {
        let license = License::new(
            "TEST-XXXX-XXXX-XXXX".to_string(),
            "http://localhost:8080".to_string(),
        );

        let result = license.validate_offline();
        assert!(result.is_err());
    }
}
