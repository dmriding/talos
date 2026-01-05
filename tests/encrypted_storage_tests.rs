use talos::client::encrypted_storage::{
    clear_license_from_disk, load_license_from_disk, save_license_to_disk,
};
use talos::client::License;
use talos::errors::LicenseError;

/// Round-trip test: save a License, load it back, and verify all fields.
#[tokio::test]
async fn test_license_encryption_storage_round_trip() {
    // Clean up any existing encrypted license before running the test
    let _ = clear_license_from_disk().await;

    // Construct a sample license using the new constructor
    let mut original = License::new(
        "TEST-LICENSE-001".to_string(),
        "http://127.0.0.1:8080".to_string(),
    );
    original.license_id = "TEST-LICENSE-001".to_string();
    original.client_id = "TEST-CLIENT-ID".to_string();
    original.hardware_id = "TEST-HARDWARE-ID".to_string();
    original.expiry_date = "2099-01-01T00:00:00Z".to_string();
    original.features = vec!["feature_a".into(), "feature_b".into()];
    original.signature = "dummy-signature".to_string();
    original.is_active = true;

    // Encrypt and store the license
    save_license_to_disk(&original)
        .await
        .expect("Encryption + storage should succeed");

    // Load and decrypt the license
    let loaded = load_license_from_disk()
        .await
        .expect("Load + decryption should succeed");

    // Verify that loaded license matches the original
    assert_eq!(loaded.license_id, original.license_id);
    assert_eq!(loaded.client_id, original.client_id);
    assert_eq!(loaded.expiry_date, original.expiry_date);
    assert_eq!(loaded.features, original.features);
    assert_eq!(loaded.server_url, original.server_url);
    assert_eq!(loaded.signature, original.signature);
    assert_eq!(loaded.is_active, original.is_active);

    // Clean up after test
    let _ = clear_license_from_disk().await;
}

/// Ensure that trying to load when no license file exists returns InvalidLicense.
#[tokio::test]
async fn test_load_without_existing_file_returns_invalid_license() {
    // Ensure file is removed
    let _ = clear_license_from_disk().await;

    let result = load_license_from_disk().await;

    match result {
        Err(LicenseError::InvalidLicense(msg)) => {
            assert!(
                msg.contains("No local license file found"),
                "Unexpected InvalidLicense message: {msg}"
            );
        }
        other => panic!("Expected InvalidLicense error, got: {:?}", other),
    }
}
