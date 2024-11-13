use talos::client::License;
use talos::hardware::get_hardware_id;

#[tokio::test]
async fn test_license_activation() {
    let mut license = License {
        license_id: "LICENSE-12345".to_string(),
        client_id: get_hardware_id(),
        expiry_date: "2025-12-31".to_string(),
        features: vec!["feature1".to_string(), "feature2".to_string()],
        server_url: "https://yourserver.com".to_string(),
        signature: "test-signature".to_string(),
        is_active: false,
    };

    // Activate the license
    let activation_result = license.activate().await;
    assert!(activation_result.is_ok(), "License activation should succeed");
    assert!(license.is_active, "License should be active after activation");
}

#[tokio::test]
async fn test_license_validation() {
    let license = License {
        license_id: "LICENSE-12345".to_string(),
        client_id: get_hardware_id(),
        expiry_date: "2025-12-31".to_string(),
        features: vec!["feature1".to_string(), "feature2".to_string()],
        server_url: "https://yourserver.com".to_string(),
        signature: "test-signature".to_string(),
        is_active: true,
    };

    // Validate the license
    let validation_result = license.validate().await;
    assert!(validation_result.is_ok(), "License validation should succeed");
}

#[tokio::test]
async fn test_license_deactivation() {
    let mut license = License {
        license_id: "LICENSE-12345".to_string(),
        client_id: get_hardware_id(),
        expiry_date: "2025-12-31".to_string(),
        features: vec!["feature1".to_string(), "feature2".to_string()],
        server_url: "https://yourserver.com".to_string(),
        signature: "test-signature".to_string(),
        is_active: true,
    };

    // Deactivate the license
    let deactivation_result = license.deactivate().await;
    assert!(deactivation_result.is_ok(), "License deactivation should succeed");
    assert!(!license.is_active, "License should not be active after deactivation");
}
