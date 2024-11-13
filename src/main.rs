use talos::key_generation::generate_secure_key;
use talos::encrypted_storage::{encrypt_and_store, load_and_decrypt};

fn main() {
    // Step 1: Generate a secure key using hardware ID, timestamp, and private key
    let secure_key = generate_secure_key().expect("Failed to generate secure key");
    println!("Generated secure key: {}", secure_key);

    // Step 2: Extract the encryption key part (the private key portion)
    let encryption_key = secure_key.split('-').last().unwrap();
    let encryption_key_bytes = hex::decode(encryption_key).expect("Failed to decode encryption key");
    
    // Step 3: Define the data to be encrypted
    let data = "Sample data to encrypt";

    // Step 4: Encrypt and store the data
    encrypt_and_store(data, &encryption_key_bytes).expect("Failed to encrypt data");

    // Step 5: Load and decrypt the data using the same key
    let decrypted_data = load_and_decrypt(&encryption_key_bytes).expect("Failed to decrypt data");
    println!("Decrypted data: {}", decrypted_data);

    // Step 6: Validate that the decrypted data matches the original
    assert_eq!(data, decrypted_data, "Data mismatch after decryption");
    println!("Encryption and decryption test passed!");
}
