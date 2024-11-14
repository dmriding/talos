use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use aes_gcm::aead::Aead;
use aes_gcm::aes::cipher::generic_array::GenericArray;
use rand::rngs::OsRng;
use rand::RngCore;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::error::Error;
use aes_gcm::aead::generic_array::typenum::U32;

const ENCRYPTED_FILE_PATH: &str = "talos_encrypted_data";

/// Generate a random encryption key (256-bit)
pub fn generate_encryption_key() -> Vec<u8> {
    let mut key = [0u8; 32]; // 256-bit key for AES-256
    OsRng.fill_bytes(&mut key); // Use OsRng from the rand crate
    key.to_vec()
}

/// Encrypt data and save it to a file
pub fn encrypt_and_store(data: &str, key: &[u8]) -> Result<(), Box<dyn Error>> {
    // Ensure key length is valid for AES-256
    if key.len() != 32 {
        return Err("Invalid key length".into());
    }

    // Initialize the cipher
    let key = GenericArray::<u8, U32>::from_slice(key);
    let cipher = Aes256Gcm::new(key);

    // Generate a random nonce (12 bytes)
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let nonce_slice = Nonce::from_slice(&nonce);

    // Encrypt the data
    let encrypted_data = cipher.encrypt(nonce_slice, data.as_bytes())
        .map_err(|_| "Encryption failed")?;

    // Save nonce and encrypted data to file
    let mut file = File::create(ENCRYPTED_FILE_PATH)?;
    file.write_all(&nonce)?;
    file.write_all(&encrypted_data)?;
    Ok(())
}

/// Decrypt data from the file
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

    // Split the contents into nonce and encrypted data
    let (nonce, encrypted_data) = contents.split_at(12);
    let nonce_slice = Nonce::from_slice(nonce);

    let key = GenericArray::<u8, U32>::from_slice(key);
    let cipher = Aes256Gcm::new(key);

    // Decrypt the data
    let decrypted_data = cipher.decrypt(nonce_slice, encrypted_data)
        .map_err(|_| "Decryption failed")?;
    
    Ok(String::from_utf8(decrypted_data)?)
}
