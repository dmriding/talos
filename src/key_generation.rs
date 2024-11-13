use crate::encrypted_storage::{generate_encryption_key, encrypt_and_store, load_and_decrypt};
use crate::hardware::get_hardware_id;
use std::time::{SystemTime, UNIX_EPOCH};
use std::error::Error;
use hex;

/// Generates and stores the private key securely
pub fn get_or_create_private_key() -> Result<Vec<u8>, Box<dyn Error>> {
    // Try to load an existing private key
    if let Ok(existing_key) = load_and_decrypt(&generate_encryption_key()) {
        return Ok(existing_key.into_bytes());
    }

    // Generate a new private key if one does not exist
    let new_key = generate_encryption_key();
    let key_str = hex::encode(&new_key);

    // Store the new key securely
    encrypt_and_store(&key_str, &new_key)?;
    Ok(new_key)
}

/// Generates a secure key using hardware ID, timestamp, and private key
pub fn generate_secure_key() -> Result<String, Box<dyn Error>> {
    // Get the hardware ID
    let hardware_id = get_hardware_id();

    // Get the current timestamp
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();

    // Get or create a private key
    let private_key = get_or_create_private_key()?;
    let private_key_encoded = hex::encode(private_key);

    // Return the combined secure key
    Ok(format!("{}-{}-{}", hardware_id, timestamp, private_key_encoded))
}
