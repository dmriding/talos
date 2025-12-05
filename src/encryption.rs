//! Symmetric encryption utilities for Talos.
//!
//! AES-256-GCM for encrypting local license data at rest.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};

use rand::rngs::OsRng;
use rand::TryRngCore;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use crate::errors::{LicenseError, LicenseResult};

/// AES-256 key size in bytes.
pub const KEY_SIZE: usize = 32;

/// GCM nonce size in bytes (96-bit).
pub const NONCE_SIZE: usize = 12;

/// Generate a new random 256-bit key.
///
/// Caller is responsible for storing this safely.
pub fn generate_key() -> [u8; KEY_SIZE] {
    let mut key = [0u8; KEY_SIZE];
    let mut rng = OsRng;

    // If OsRng fails here, the environment is badly broken → hard panic is acceptable.
    rng.try_fill_bytes(&mut key)
        .expect("OsRng failed to generate encryption key");

    key
}

/// Encrypt arbitrary bytes using AES-256-GCM.
///
/// Output format:
///   [nonce (12 bytes)] || [ciphertext+tag]
pub fn encrypt_bytes(plaintext: &[u8], key: &[u8]) -> LicenseResult<Vec<u8>> {
    if key.len() != KEY_SIZE {
        return Err(LicenseError::EncryptionError(format!(
            "invalid key length: expected {} bytes, got {}",
            KEY_SIZE,
            key.len()
        )));
    }

    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);

    // Generate a random nonce (12 bytes)
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    let mut rng = OsRng;
    rng.try_fill_bytes(&mut nonce_bytes)
        .expect("OsRng failed to generate nonce");
    let nonce = Nonce::from_slice(&nonce_bytes);

    let mut ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| LicenseError::EncryptionError(format!("encryption failed: {e}")))?;

    let mut output = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    output.extend_from_slice(&nonce_bytes);
    output.append(&mut ciphertext);

    Ok(output)
}

/// Decrypt bytes produced by `encrypt_bytes`.
pub fn decrypt_bytes(ciphertext: &[u8], key: &[u8]) -> LicenseResult<Vec<u8>> {
    if key.len() != KEY_SIZE {
        return Err(LicenseError::DecryptionError(format!(
            "invalid key length: expected {} bytes, got {}",
            KEY_SIZE,
            key.len()
        )));
    }

    if ciphertext.len() <= NONCE_SIZE {
        return Err(LicenseError::DecryptionError(
            "ciphertext too short".to_string(),
        ));
    }

    let (nonce_bytes, ct) = ciphertext.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);

    let plaintext = cipher
        .decrypt(nonce, ct)
        .map_err(|e| LicenseError::DecryptionError(format!("decryption failed: {e}")))?;

    Ok(plaintext)
}

/// Encrypt bytes and return a Base64 string.
pub fn encrypt_to_base64(plaintext: &[u8], key: &[u8]) -> LicenseResult<String> {
    let encrypted = encrypt_bytes(plaintext, key)?;
    Ok(B64.encode(encrypted))
}

/// Decrypt a Base64 ciphertext previously produced by `encrypt_to_base64`.
pub fn decrypt_from_base64(ciphertext_b64: &str, key: &[u8]) -> LicenseResult<Vec<u8>> {
    let decoded = B64
        .decode(ciphertext_b64)
        .map_err(|e| LicenseError::DecryptionError(format!("base64 decode failed: {e}")))?;
    decrypt_bytes(&decoded, key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_encrypt_decrypt_bytes() {
        let key = generate_key();
        let data = b"hello talos encryption";

        let encrypted = encrypt_bytes(data, &key).expect("encryption should succeed");
        assert_ne!(encrypted, data, "ciphertext must differ from plaintext");

        let decrypted = decrypt_bytes(&encrypted, &key).expect("decryption should succeed");
        assert_eq!(decrypted, data);
    }

    #[test]
    fn round_trip_encrypt_decrypt_base64() {
        let key = generate_key();
        let data = b"talos base64 test";

        let encoded = encrypt_to_base64(data, &key).expect("encryption should succeed");
        let decoded = decrypt_from_base64(&encoded, &key).expect("decryption should succeed");

        assert_eq!(decoded, data);
    }

    #[test]
    fn rejects_wrong_key_size() {
        let key = [0u8; 16]; // too short
        let data = b"test";

        let enc = encrypt_bytes(data, &key);
        assert!(enc.is_err());

        let dec = decrypt_bytes(&[0u8; NONCE_SIZE + 16], &key);
        assert!(dec.is_err());
    }
}
