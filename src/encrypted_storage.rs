use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use aes_gcm::aead::Aead;
use aes_gcm::aes::cipher::generic_array::GenericArray;
use ring::rand::{SystemRandom, SecureRandom};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::error::Error;
use aes_gcm::aead::generic_array::typenum::U32;

/// Path where the encrypted data will be stored
const ENCRYPTED_FILE_PATH: &str = "talos_encrypted_data";

/// Generate a random encryption key
pub fn generate_encryption_key() -> Vec<u8> {
    let mut key = [0u8; 32]; // 256-bit key for AES-256
    let rng = ring::rand::SystemRandom::new();
    rng.fill(&mut key).expect("Failed to generate encryption key");
    key.to_vec()
}

/// Encrypt data and save it to a file
pub fn encrypt_and_store(data: &str, key: &[u8]) -> Result<(), Box<dyn Error>> {
    if key.len() != 32 {
        return Err("Invalid key length".into());
    }

    // Initialize the cipher with the given key using GenericArray
    let key = GenericArray::<u8, U32>::from_slice(key);
    let cipher = Aes256Gcm::new(key);

    // Generate a random nonce
    let mut nonce = [0u8; 12];
    let rng = SystemRandom::new();
    rng.fill(&mut nonce).map_err(|_| "Failed to generate nonce")?;
    let nonce = Nonce::from_slice(&nonce);

    // Encrypt the data
    let encrypted_data = cipher.encrypt(nonce, data.as_bytes())
        .map_err(|_| "Encryption failed")?;

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

    if key.len() != 32 {
        return Err("Invalid key length".into());
    }

    let mut file = File::open(ENCRYPTED_FILE_PATH)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;

    let (nonce, encrypted_data) = contents.split_at(12);
    let nonce = Nonce::from_slice(nonce);

    let key = GenericArray::<u8, U32>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    
    let decrypted_data = cipher.decrypt(nonce, encrypted_data)
        .map_err(|_| "Decryption failed")?;
    Ok(String::from_utf8(decrypted_data)?)
}
