//! Secure cached validation state for offline/air-gapped systems.
//!
//! This module provides encrypted, hardware-bound storage for license validation
//! state that can be used for offline validation during grace periods.
//!
//! ## Security Model
//!
//! The cached validation data is protected by:
//!
//! 1. **AES-256-GCM encryption** - Data is encrypted at rest
//! 2. **Hardware binding** - Encryption key is derived from hardware fingerprint
//! 3. **Tamper detection** - GCM authentication tag prevents modification
//! 4. **Server authority** - Grace period comes from server, cannot be forged
//!
//! A user cannot:
//! - Read the cache contents without the hardware-bound key
//! - Modify the cache (authentication tag would fail)
//! - Copy the cache to another machine (different hardware = different key)
//! - Extend the grace period (server-provided, stored encrypted)

use crate::client::storage::{clear_from_storage, load_from_storage, save_to_storage, StorageKey};
use crate::encryption::{decrypt_from_base64, encrypt_to_base64, KEY_SIZE};
use crate::errors::{LicenseError, LicenseResult};
use crate::hardware::get_hardware_id;

use chrono::{DateTime, Utc};
use ring::digest::{digest, SHA256};
use serde::{Deserialize, Serialize};

/// Cached validation state for offline use.
///
/// This struct stores the essential license validation data that can be used
/// to validate a license offline during a grace period.
///
/// **Security Note:** All fields are server-provided and stored encrypted.
/// The client cannot forge or modify this data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedValidation {
    /// The license key this cache belongs to
    pub license_key: String,

    /// Hardware ID this cache is bound to (for verification)
    pub hardware_id: String,

    /// List of features enabled for this license
    pub features: Vec<String>,

    /// License tier name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,

    /// License expiration date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,

    /// When the offline grace period ends (ISO 8601)
    ///
    /// After this time, the license must be validated online.
    /// This value comes from the server and cannot be modified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_ends_at: Option<String>,

    /// When this cache was last updated from the server (ISO 8601)
    pub validated_at: String,
}

impl CachedValidation {
    /// Create a new cached validation from server response data.
    pub fn new(
        license_key: String,
        hardware_id: String,
        features: Vec<String>,
        tier: Option<String>,
        expires_at: Option<String>,
        grace_period_ends_at: Option<String>,
    ) -> Self {
        Self {
            license_key,
            hardware_id,
            features,
            tier,
            expires_at,
            grace_period_ends_at,
            validated_at: Utc::now().to_rfc3339(),
        }
    }

    /// Check if this cache is still valid for offline use.
    ///
    /// Returns `true` if:
    /// - The cache has a grace period that hasn't expired yet, OR
    /// - The cache has no grace period (normal online license)
    ///
    /// Returns `false` if:
    /// - The grace period has expired (must go online)
    pub fn is_valid_for_offline(&self) -> bool {
        match &self.grace_period_ends_at {
            Some(ends_at) => {
                // Parse the grace period end time
                match DateTime::parse_from_rfc3339(ends_at) {
                    Ok(end_time) => Utc::now() < end_time.with_timezone(&Utc),
                    // If we can't parse it, assume invalid (fail safe)
                    Err(_) => false,
                }
            }
            // No grace period means this is a normal license, not air-gapped
            // For safety, require online validation
            None => false,
        }
    }

    /// Check if the license itself has expired (separate from grace period).
    pub fn is_license_expired(&self) -> bool {
        match &self.expires_at {
            Some(expires) => match DateTime::parse_from_rfc3339(expires) {
                Ok(exp_time) => Utc::now() >= exp_time.with_timezone(&Utc),
                Err(_) => false, // Can't parse, assume not expired
            },
            None => false, // No expiration = never expires
        }
    }

    /// Get time remaining until grace period ends.
    ///
    /// Returns `None` if no grace period or if it has already expired.
    pub fn grace_period_remaining(&self) -> Option<chrono::Duration> {
        self.grace_period_ends_at.as_ref().and_then(|ends_at| {
            DateTime::parse_from_rfc3339(ends_at)
                .ok()
                .and_then(|end_time| {
                    let remaining = end_time.with_timezone(&Utc) - Utc::now();
                    if remaining > chrono::Duration::zero() {
                        Some(remaining)
                    } else {
                        None
                    }
                })
        })
    }

    /// Check if this cache belongs to the current hardware.
    pub fn matches_hardware(&self) -> bool {
        self.hardware_id == get_hardware_id()
    }

    /// Check if a specific feature is enabled.
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }
}

// === Storage Functions ===

/// Derive a 256-bit storage key from the hardware ID.
///
/// This key is:
/// - Unique to this device (hardware-bound)
/// - Never sent to the server
/// - Used only for local at-rest encryption
fn derive_cache_storage_key() -> [u8; KEY_SIZE] {
    let hw_id = get_hardware_id();
    // Add a salt to differentiate from license storage key
    let salted = format!("talos_cache_v1:{}", hw_id);
    let hash = digest(&SHA256, salted.as_bytes());

    let mut key = [0u8; KEY_SIZE];
    key.copy_from_slice(hash.as_ref());
    key
}

/// Save cached validation to secure storage.
///
/// The cache is:
/// - Serialized to JSON
/// - Encrypted with AES-256-GCM using a hardware-bound key
/// - Stored in OS keyring (or app data directory as fallback)
pub async fn save_cache_to_disk(cache: &CachedValidation) -> LicenseResult<()> {
    let key = derive_cache_storage_key();

    let json_bytes = serde_json::to_vec(cache)
        .map_err(|e| LicenseError::EncryptionError(format!("Failed to serialize cache: {e}")))?;

    let encrypted_b64 = encrypt_to_base64(&json_bytes, &key)?;

    save_to_storage(StorageKey::Cache, &encrypted_b64).await
}

/// Load cached validation from secure storage.
///
/// Checks storage locations in order:
/// 1. OS keyring
/// 2. App data directory file
/// 3. Legacy CWD file (with automatic migration)
///
/// Returns an error if:
/// - No cache is found in any location
/// - Decryption fails (wrong key, tampered data)
/// - Deserialization fails
/// - Cache doesn't match current hardware
pub async fn load_cache_from_disk() -> LicenseResult<CachedValidation> {
    let encrypted_b64 = load_from_storage(StorageKey::Cache).await?;

    let key = derive_cache_storage_key();

    let decrypted_bytes = decrypt_from_base64(encrypted_b64.trim(), &key)?;

    let cache: CachedValidation = serde_json::from_slice(&decrypted_bytes)
        .map_err(|e| LicenseError::DecryptionError(format!("Failed to deserialize cache: {e}")))?;

    // Verify the cache belongs to this hardware
    if !cache.matches_hardware() {
        return Err(LicenseError::InvalidLicense(
            "Cache does not match current hardware.".to_string(),
        ));
    }

    Ok(cache)
}

/// Delete cached validation from all storage locations.
///
/// Clears from keyring, app data directory, and legacy CWD location.
pub async fn clear_cache_from_disk() -> LicenseResult<()> {
    clear_from_storage(StorageKey::Cache).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use serial_test::serial;
    use std::io::ErrorKind;
    use std::path::Path;
    use tokio::fs;
    use tokio::test as tokio_test;

    // Test-specific file paths for encryption layer tests
    const TEST_CACHE_FILE_1: &str = "talos_cache_test_roundtrip.enc";
    const TEST_CACHE_FILE_2: &str = "talos_cache_test_tamper.enc";

    async fn cleanup_test_file(path: &str) {
        let _ = fs::remove_file(Path::new(path)).await;
    }

    // Helper to clear cache from storage
    async fn cleanup() {
        let _ = clear_cache_from_disk().await;
    }

    // Helper to save cache to a specific test file (for encryption layer tests)
    async fn save_cache_to_test_file(cache: &CachedValidation, path: &str) -> LicenseResult<()> {
        let key = derive_cache_storage_key();

        let json_bytes = serde_json::to_vec(cache).map_err(|e| {
            LicenseError::EncryptionError(format!("Failed to serialize cache: {e}"))
        })?;

        let encrypted_b64 = encrypt_to_base64(&json_bytes, &key)?;

        fs::write(Path::new(path), encrypted_b64).await?;

        Ok(())
    }

    // Helper to load cache from a specific test file (for encryption layer tests)
    async fn load_cache_from_test_file(path: &str) -> LicenseResult<CachedValidation> {
        let encrypted_b64 = match fs::read_to_string(Path::new(path)).await {
            Ok(s) => s,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                return Err(LicenseError::InvalidLicense(
                    "No cached validation found.".to_string(),
                ));
            }
            Err(e) => return Err(LicenseError::StorageError(e)),
        };

        let key = derive_cache_storage_key();

        let decrypted_bytes = decrypt_from_base64(encrypted_b64.trim(), &key)?;

        let cache: CachedValidation = serde_json::from_slice(&decrypted_bytes).map_err(|e| {
            LicenseError::DecryptionError(format!("Failed to deserialize cache: {e}"))
        })?;

        // Verify the cache belongs to this hardware
        if !cache.matches_hardware() {
            return Err(LicenseError::InvalidLicense(
                "Cache does not match current hardware.".to_string(),
            ));
        }

        Ok(cache)
    }

    fn create_test_cache(grace_period_hours: Option<i64>) -> CachedValidation {
        let grace_period_ends_at =
            grace_period_hours.map(|hours| (Utc::now() + Duration::hours(hours)).to_rfc3339());

        CachedValidation {
            license_key: "TEST-XXXX-XXXX-XXXX".to_string(),
            hardware_id: get_hardware_id(),
            features: vec!["feature_a".to_string(), "feature_b".to_string()],
            tier: Some("pro".to_string()),
            expires_at: Some((Utc::now() + Duration::days(365)).to_rfc3339()),
            grace_period_ends_at,
            validated_at: Utc::now().to_rfc3339(),
        }
    }

    #[tokio_test]
    async fn round_trip_cache_encrypt_decrypt() {
        cleanup_test_file(TEST_CACHE_FILE_1).await;

        let cache = create_test_cache(Some(24)); // 24 hours grace period

        save_cache_to_test_file(&cache, TEST_CACHE_FILE_1)
            .await
            .expect("save should succeed");

        let loaded = load_cache_from_test_file(TEST_CACHE_FILE_1)
            .await
            .expect("load should succeed");

        assert_eq!(loaded.license_key, cache.license_key);
        assert_eq!(loaded.hardware_id, cache.hardware_id);
        assert_eq!(loaded.features, cache.features);
        assert_eq!(loaded.tier, cache.tier);
        assert_eq!(loaded.grace_period_ends_at, cache.grace_period_ends_at);

        cleanup_test_file(TEST_CACHE_FILE_1).await;
    }

    #[tokio_test]
    #[serial]
    async fn missing_cache_returns_error() {
        cleanup().await;

        let result = load_cache_from_disk().await;
        assert!(matches!(result, Err(LicenseError::InvalidLicense(_))));
    }

    #[tokio_test]
    #[serial]
    async fn storage_api_round_trip() {
        cleanup().await;

        let cache = create_test_cache(Some(24));

        save_cache_to_disk(&cache)
            .await
            .expect("save should succeed");

        let loaded = load_cache_from_disk().await.expect("load should succeed");

        assert_eq!(loaded.license_key, cache.license_key);
        assert_eq!(loaded.hardware_id, cache.hardware_id);
        assert_eq!(loaded.features, cache.features);
        assert_eq!(loaded.tier, cache.tier);

        cleanup().await;
    }

    #[test]
    fn cache_with_valid_grace_period() {
        let cache = create_test_cache(Some(24)); // 24 hours in future
        assert!(cache.is_valid_for_offline());
        assert!(cache.grace_period_remaining().is_some());
    }

    #[test]
    fn cache_with_expired_grace_period() {
        let mut cache = create_test_cache(None);
        // Set grace period to 1 hour in the past
        cache.grace_period_ends_at = Some((Utc::now() - Duration::hours(1)).to_rfc3339());

        assert!(!cache.is_valid_for_offline());
        assert!(cache.grace_period_remaining().is_none());
    }

    #[test]
    fn cache_without_grace_period() {
        let cache = create_test_cache(None);
        // No grace period = not valid for offline (fail safe)
        assert!(!cache.is_valid_for_offline());
    }

    #[test]
    fn cache_license_expired() {
        let mut cache = create_test_cache(Some(24));
        cache.expires_at = Some((Utc::now() - Duration::days(1)).to_rfc3339());

        assert!(cache.is_license_expired());
    }

    #[test]
    fn cache_license_not_expired() {
        let cache = create_test_cache(Some(24));
        assert!(!cache.is_license_expired());
    }

    #[test]
    fn cache_has_feature() {
        let cache = create_test_cache(Some(24));
        assert!(cache.has_feature("feature_a"));
        assert!(cache.has_feature("feature_b"));
        assert!(!cache.has_feature("feature_c"));
    }

    #[test]
    fn cache_matches_hardware() {
        let cache = create_test_cache(Some(24));
        assert!(cache.matches_hardware());

        let mut wrong_hw_cache = cache.clone();
        wrong_hw_cache.hardware_id = "wrong-hardware-id".to_string();
        assert!(!wrong_hw_cache.matches_hardware());
    }

    #[tokio_test]
    async fn tampered_cache_fails_to_load() {
        cleanup_test_file(TEST_CACHE_FILE_2).await;

        let cache = create_test_cache(Some(24));
        save_cache_to_test_file(&cache, TEST_CACHE_FILE_2)
            .await
            .expect("save should succeed");

        // Read the encrypted file
        let encrypted = fs::read_to_string(Path::new(TEST_CACHE_FILE_2))
            .await
            .expect("read should succeed");

        // Tamper with the data by modifying a character
        let mut tampered = encrypted.clone();
        if let Some(c) = tampered.pop() {
            // Change the last character
            let new_char = if c == 'A' { 'B' } else { 'A' };
            tampered.push(new_char);
        }

        // Write tampered data back
        fs::write(Path::new(TEST_CACHE_FILE_2), tampered)
            .await
            .expect("write should succeed");

        // Try to load - should fail due to authentication tag mismatch
        let result = load_cache_from_test_file(TEST_CACHE_FILE_2).await;
        assert!(result.is_err());

        cleanup_test_file(TEST_CACHE_FILE_2).await;
    }
}
