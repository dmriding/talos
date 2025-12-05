use aes_gcm::aead::Aead;
use aes_gcm::aes::cipher::generic_array::GenericArray;
use aes_gcm::aead::generic_array::typenum::U32;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use rand::rngs::OsRng;
use rand::TryRngCore;
use std::error::Error;
use std::fs::{File, remove_file};
use std::io::{Read, Write};
use std::path::Path;

const ENCRYPTED_FILE_PATH: &str = "talos_encrypted_data";

/// Generate a random encryption key (256-bit).
pub fn generate_encryption_key() -> Vec<u8> {
    let mut key = [0u8; 32]; // 256-bit key for AES-256
    let mut rng = OsRng;
    rng.try_fill_bytes(&mut key)
        .expect("OsRng failed to generate encrypted_storage key");
    key.to_vec()
}

/// Encrypt data and save it to a file.
///
/// File layout:
/// - First 12 bytes: nonce
/// - Remaining bytes: ciphertext+tag
pub fn encrypt_and_store(data: &str, key: &[u8]) -> Result<(), Box<dyn Error>> {
    // Ensure key length is valid for AES-256
    if key.len() != 32 {
        return Err("Invalid key length".into());
    }

    // Initialize the cipher
    let key_array = GenericArray::<u8, U32>::from_slice(key);
    let cipher = Aes256Gcm::new(key_array);

    // Generate a random nonce (12 bytes)
    let mut nonce = [0u8; 12];
    let mut rng = OsRng;
    rng.try_fill_bytes(&mut nonce)
        .expect("OsRng failed to generate nonce for encrypted_storage");
    let nonce_slice = Nonce::from_slice(&nonce);

    // Encrypt the data
    let encrypted_data = cipher
        .encrypt(nonce_slice, data.as_bytes())
        .map_err(|_| "Encryption failed")?;

    // Save nonce and encrypted data to file
    let mut file = File::create(ENCRYPTED_FILE_PATH)?;
    file.write_all(&nonce)?;
    file.write_all(&encrypted_data)?;
    Ok(())
}

/// Decrypt data from the file created by `encrypt_and_store`.
pub fn load_and_decrypt(key: &[u8]) -> Result<String, Box<dyn Error>> {
    if !Path::new(ENCRYPTED_FILE_PATH).exists() {
        return Err("Encrypted file not found".into());
    }

    // Ensure key length is valid for AES-256
    if key.len() != 32 {
        return Err("Invalid key length".into());
    }

    let mut file = File::open(ENCRYPTED_FILE_PATH)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;

    if contents.len() <= 12 {
        return Err("Encrypted file is too short".into());
    }

    // Split the contents into nonce and encrypted data
    let (nonce, encrypted_data) = contents.split_at(12);
    let nonce_slice = Nonce::from_slice(nonce);

    let key_array = GenericArray::<u8, U32>::from_slice(key);
    let cipher = Aes256Gcm::new(key_array);

    // Decrypt the data
    let decrypted_data = cipher
        .decrypt(nonce_slice, encrypted_data)
        .map_err(|_| "Decryption failed")?;

    Ok(String::from_utf8(decrypted_data)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Helper to clean up the test file before/after tests.
    fn cleanup_file() {
        if Path::new(ENCRYPTED_FILE_PATH).exists() {
            let _ = remove_file(ENCRYPTED_FILE_PATH);
        }
    }

    #[test]
    fn round_trip_encrypt_and_decrypt() {
        cleanup_file();

        let key = generate_encryption_key();
        let original = "talos encrypted storage test";

        encrypt_and_store(original, &key).expect("encryption + store should succeed");

        let decrypted = load_and_decrypt(&key).expect("load + decrypt should succeed");

        assert_eq!(decrypted, original);

        cleanup_file();
    }

    #[test]
    fn invalid_key_length_rejected_on_encrypt() {
        cleanup_file();

        let bad_key = vec![0u8; 16]; // too short
        let result = encrypt_and_store("data", &bad_key);

        assert!(result.is_err());

        cleanup_file();
    }

    #[test]
    fn invalid_key_length_rejected_on_decrypt() {
        cleanup_file();

        let key = generate_encryption_key();
        encrypt_and_store("data", &key).expect("encrypt should succeed");

        let bad_key = vec![0u8; 16]; // too short
        let result = load_and_decrypt(&bad_key);

        assert!(result.is_err());

        cleanup_file();
    }

    #[test]
    fn missing_file_returns_error() {
        cleanup_file();

        let key = generate_encryption_key();
        let result = load_and_decrypt(&key);

        assert!(result.is_err());
    }

    #[test]
    fn corrupt_file_is_rejected() {
        cleanup_file();

        // Write a file that's too short / invalid
        {
            let mut file = File::create(ENCRYPTED_FILE_PATH).expect("create file");
            file.write_all(b"short").expect("write file");
        }

        let key = generate_encryption_key();
        let result = load_and_decrypt(&key);

        assert!(result.is_err());

        cleanup_file();
    }
}
