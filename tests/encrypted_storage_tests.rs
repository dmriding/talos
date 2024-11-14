use std::fs;
use std::path::Path;
use talos::client::encrypted_storage::{generate_encryption_key, encrypt_and_store, load_and_decrypt};

const ENCRYPTED_FILE_PATH: &str = "talos_encrypted_data";

#[test]
fn test_encryption_storage() {
    // Step 1: Clean up any existing encrypted file before running the test
    if Path::new(ENCRYPTED_FILE_PATH).exists() {
        let _ = fs::remove_file(ENCRYPTED_FILE_PATH);
    }

    // Step 2: Generate a 32-byte encryption key
    let encryption_key = generate_encryption_key();
    assert_eq!(encryption_key.len(), 32, "Encryption key should be 32 bytes long");

    // Step 3: Define the data to be encrypted
    let data = "Test data for encryption";

    // Step 4: Encrypt and store the data
    let result = encrypt_and_store(data, &encryption_key);
    assert!(result.is_ok(), "Encryption and storage should succeed");

    // Ensure the encrypted file exists
    assert!(Path::new(ENCRYPTED_FILE_PATH).exists(), "Encrypted file should exist");

    // Step 5: Load and decrypt the data using the same key
    let decrypted_data = load_and_decrypt(&encryption_key)
        .expect("Failed to decrypt data");

    // Step 6: Ensure the decrypted data matches the original
    assert_eq!(data, decrypted_data, "Decrypted data should match the original data");

    // Step 7: Clean up the file after the test
    let _ = fs::remove_file(ENCRYPTED_FILE_PATH);
}
