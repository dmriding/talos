use talos::encrypted_storage::{generate_encryption_key, encrypt_and_store, load_and_decrypt};

#[test]
fn test_encryption_storage() {
    // Step 1: Generate a 32-byte encryption key
    let encryption_key = generate_encryption_key();
    assert_eq!(encryption_key.len(), 32, "Encryption key should be 32 bytes long");

    // Step 2: Define the data to be encrypted
    let data = "Test data for encryption";

    // Step 3: Encrypt and store the data
    let result = encrypt_and_store(data, &encryption_key);
    assert!(result.is_ok(), "Encryption and storage should succeed");

    // Step 4: Load and decrypt the data using the same key
    let decrypted_data = load_and_decrypt(&encryption_key)
        .expect("Failed to decrypt data");

    // Step 5: Ensure the decrypted data matches the original
    assert_eq!(data, decrypted_data, "Decrypted data should match the original data");
}
