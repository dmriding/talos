use crate::encryption::{decrypt_from_base64, encrypt_to_base64, generate_key, KEY_SIZE};
use crate::errors::{LicenseError, LicenseResult};
use crate::hardware::get_hardware_id;

use hex;
use ring::digest::{digest, SHA256};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// File where the device-specific private key is stored (encrypted).
const PRIVATE_KEY_FILE: &str = "talos_private_key.enc";

/// Derive a 256-bit key used only for encrypting the private key at rest.
///
/// This is *not* the same as the generated private key itself.
/// It is a storage key derived from hardware ID:
///   storage_key = SHA256(get_hardware_id())
fn derive_private_key_storage_key() -> [u8; KEY_SIZE] {
    let hw_id = get_hardware_id();
    let hash = digest(&SHA256, hw_id.as_bytes());

    let mut key = [0u8; KEY_SIZE];
    key.copy_from_slice(hash.as_ref());
    key
}

/// Load the private key from disk (if it exists).
///
/// Returns:
/// - `Ok(Some(Vec<u8>))` if a key exists and is decrypted successfully.
/// - `Ok(None)` if the key file does not exist.
/// - `Err(LicenseError::StorageError | EncryptionError | DecryptionError)` otherwise.
fn load_private_key_from_disk() -> LicenseResult<Option<Vec<u8>>> {
    let path = Path::new(PRIVATE_KEY_FILE);

    let encrypted_b64 = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            // No key stored yet.
            return Ok(None);
        }
        Err(e) => {
            return Err(LicenseError::StorageError(e));
        }
    };

    let storage_key = derive_private_key_storage_key();
    let decrypted_bytes = decrypt_from_base64(encrypted_b64.trim(), &storage_key)?;

    Ok(Some(decrypted_bytes))
}

/// Store the private key on disk, encrypted with the per-device storage key.
fn store_private_key_to_disk(private_key: &[u8]) -> LicenseResult<()> {
    let path = Path::new(PRIVATE_KEY_FILE);
    let storage_key = derive_private_key_storage_key();

    let encrypted_b64 = encrypt_to_base64(private_key, &storage_key)?;
    fs::write(path, encrypted_b64)?;

    Ok(())
}

/// Get an existing private key, or generate + store a new one if none exists.
///
/// The private key itself is:
/// - random 256-bit (via `generate_key()`),
/// - stored encrypted at rest using a key derived from the hardware ID.
pub fn get_or_create_private_key() -> LicenseResult<Vec<u8>> {
    // Try to load an existing private key.
    if let Some(existing_key) = load_private_key_from_disk()? {
        println!("Loaded existing private key.");
        return Ok(existing_key);
    }

    // Generate a new random private key.
    let new_key = generate_key(); // [u8; KEY_SIZE]
    store_private_key_to_disk(&new_key)?;
    println!("Generated and stored new private key.");

    Ok(new_key.to_vec())
}

/// Generates a secure key using:
/// - hardware ID,
/// - current timestamp,
/// - device-specific private key.
///
/// Format:
///   "{hardware_id}-{unix_timestamp}-{hex(private_key)}"
pub fn generate_secure_key() -> LicenseResult<String> {
    // Get the hardware ID.
    let hardware_id = get_hardware_id();

    // Get the current timestamp.
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| LicenseError::ConfigError(format!("system time error: {e}")))?
        .as_secs();

    // Get or create a device-specific private key.
    let private_key = get_or_create_private_key()?;
    let private_key_encoded = hex::encode(&private_key);

    // Build the composite secure key.
    let secure_key = format!("{}-{}-{}", hardware_id, timestamp, private_key_encoded);
    println!("Generated secure key: {}", secure_key);

    Ok(secure_key)
}
