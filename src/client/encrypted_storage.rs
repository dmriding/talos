use crate::client::client::License;
use crate::encryption::{decrypt_from_base64, encrypt_to_base64, KEY_SIZE};
use crate::errors::{LicenseError, LicenseResult};
use crate::hardware::get_hardware_id;

use ring::digest::{digest, SHA256};
use serde_json;
use std::io::ErrorKind;
use std::path::Path;
use tokio::fs;

const LICENSE_STORAGE_FILE: &str = "talos_license.enc";

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

/// Encrypt and write the license JSON to disk using the shared encryption module.
///
/// Format on disk:
///   base64( nonce || ciphertext+tag )
pub async fn save_license_to_disk(license: &License) -> LicenseResult<()> {
    let key = derive_local_storage_key();

    let json_bytes = serde_json::to_vec(license).map_err(|e| {
        LicenseError::EncryptionError(format!("Failed to serialize license for storage: {e}"))
    })?;

    // AES-256-GCM + nonce + base64 handled by encryption module.
    let encrypted_b64 = encrypt_to_base64(&json_bytes, &key)?;

    // Filesystem errors bubble as LicenseError::StorageError via `?`.
    fs::write(Path::new(LICENSE_STORAGE_FILE), encrypted_b64).await?;

    Ok(())
}

/// Read, decrypt, and deserialize the license from disk.
///
/// Errors:
/// - `InvalidLicense` if the file is missing.
/// - `StorageError` for I/O failures.
/// - `DecryptionError` / `EncryptionError` for crypto issues.
pub async fn load_license_from_disk() -> LicenseResult<License> {
    let encrypted_b64 = match fs::read_to_string(Path::new(LICENSE_STORAGE_FILE)).await {
        Ok(s) => s,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Err(LicenseError::InvalidLicense(
                "No local license file found.".to_string(),
            ));
        }
        Err(e) => return Err(LicenseError::StorageError(e)),
    };

    let key = derive_local_storage_key();

    let decrypted_bytes = decrypt_from_base64(encrypted_b64.trim(), &key)?;

    let license: License = serde_json::from_slice(&decrypted_bytes).map_err(|e| {
        LicenseError::DecryptionError(format!(
            "Failed to deserialize license from decrypted bytes: {e}"
        ))
    })?;

    Ok(license)
}

/// Delete the local license file if it exists (no-op if missing).
pub async fn clear_license_from_disk() -> LicenseResult<()> {
    match fs::remove_file(Path::new(LICENSE_STORAGE_FILE)).await {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(LicenseError::StorageError(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tokio::test as tokio_test;

    // Simple helper to remove the license file if present.
    async fn cleanup_file() {
        let _ = clear_license_from_disk().await;
    }

    // Use serial test attribute to prevent race conditions between tests
    // that share the same LICENSE_STORAGE_FILE
    #[tokio_test]
    #[serial]
    async fn round_trip_license_encrypt_decrypt() {
        cleanup_file().await;

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

        cleanup_file().await;
    }

    #[tokio_test]
    #[serial]
    async fn missing_file_returns_invalid_license() {
        cleanup_file().await;

        let result = load_license_from_disk().await;
        assert!(
            matches!(result, Err(LicenseError::InvalidLicense(_))),
            "Expected InvalidLicense error, got: {:?}",
            result
        );
    }
}
