use crate::client::client::License;
use crate::client::storage::{clear_from_storage, load_from_storage, save_to_storage, StorageKey};
use crate::encryption::{decrypt_from_base64, encrypt_to_base64, KEY_SIZE};
use crate::errors::{LicenseError, LicenseResult};
use crate::hardware::get_hardware_id;

use ring::digest::{digest, SHA256};
use serde_json;

/// Derive a 256-bit local storage key from the hardware ID using SHA-256.
///
/// This key:
/// - is used only for local at-rest encryption of the license,
/// - is never sent to the server,
/// - is stable per device (as long as `get_hardware_id()` is stable).
fn derive_local_storage_key() -> [u8; KEY_SIZE] {
    let hw_id = get_hardware_id();
    let hash = digest(&SHA256, hw_id.as_bytes());

    let mut key = [0u8; KEY_SIZE];
    key.copy_from_slice(hash.as_ref());
    key
}

/// Encrypt and save the license using the secure storage backend.
///
/// Data is encrypted with AES-256-GCM using a hardware-derived key,
/// then stored in the OS keyring (or app data directory as fallback).
///
/// Format stored:
///   base64( nonce || ciphertext+tag )
pub async fn save_license_to_disk(license: &License) -> LicenseResult<()> {
    let key = derive_local_storage_key();

    let json_bytes = serde_json::to_vec(license).map_err(|e| {
        LicenseError::EncryptionError(format!("Failed to serialize license for storage: {e}"))
    })?;

    // AES-256-GCM + nonce + base64 handled by encryption module.
    let encrypted_b64 = encrypt_to_base64(&json_bytes, &key)?;

    // Save to secure storage (keyring with file fallback)
    save_to_storage(StorageKey::License, &encrypted_b64).await
}

/// Load, decrypt, and deserialize the license from secure storage.
///
/// Checks storage locations in order:
/// 1. OS keyring
/// 2. App data directory file
/// 3. Legacy CWD file (with automatic migration)
///
/// Errors:
/// - `InvalidLicense` if no stored license is found.
/// - `StorageError` for I/O failures.
/// - `DecryptionError` / `EncryptionError` for crypto issues.
pub async fn load_license_from_disk() -> LicenseResult<License> {
    let encrypted_b64 = load_from_storage(StorageKey::License).await?;

    let key = derive_local_storage_key();

    let decrypted_bytes = decrypt_from_base64(encrypted_b64.trim(), &key)?;

    let license: License = serde_json::from_slice(&decrypted_bytes).map_err(|e| {
        LicenseError::DecryptionError(format!(
            "Failed to deserialize license from decrypted bytes: {e}"
        ))
    })?;

    Ok(license)
}

/// Delete the stored license from all storage locations.
///
/// Clears from keyring, app data directory, and legacy CWD location.
pub async fn clear_license_from_disk() -> LicenseResult<()> {
    clear_from_storage(StorageKey::License).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tokio::test as tokio_test;

    // Simple helper to clear license from storage.
    async fn cleanup() {
        let _ = clear_license_from_disk().await;
    }

    // Use serial test attribute to prevent race conditions between tests
    // that share the same storage location
    #[tokio_test]
    #[serial]
    async fn round_trip_license_encrypt_decrypt() {
        cleanup().await;

        let mut license =
            License::new("TEST-XXXX-XXXX-XXXX".into(), "http://localhost:8080".into());
        // Set some fields for testing
        license.license_id = "LIC-123".into();
        license.client_id = "CLIENT-XYZ".into();
        license.hardware_id = "CLIENT-XYZ".into();
        license.expiry_date = "2099-01-01T00:00:00Z".into();
        license.features = vec!["feature_a".into(), "feature_b".into()];
        license.signature = "dummy-signature".into();
        license.is_active = true;

        save_license_to_disk(&license)
            .await
            .expect("save should succeed");

        let loaded = load_license_from_disk().await.expect("load should succeed");

        assert_eq!(loaded.license_key, license.license_key);
        assert_eq!(loaded.license_id, license.license_id);
        assert_eq!(loaded.client_id, license.client_id);
        assert_eq!(loaded.hardware_id, license.hardware_id);
        assert_eq!(loaded.expiry_date, license.expiry_date);
        assert_eq!(loaded.features, license.features);
        assert_eq!(loaded.server_url, license.server_url);
        assert_eq!(loaded.signature, license.signature);
        assert_eq!(loaded.is_active, license.is_active);

        cleanup().await;
    }

    #[tokio_test]
    #[serial]
    async fn missing_file_returns_invalid_license() {
        cleanup().await;

        let result = load_license_from_disk().await;
        assert!(
            matches!(result, Err(LicenseError::InvalidLicense(_))),
            "Expected InvalidLicense error, got: {:?}",
            result
        );
    }
}
