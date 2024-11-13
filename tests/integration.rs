use talos::client::License;

#[tokio::test]
async fn integration_test_license_lifecycle() {
    let mut license = License {
        license_id: "LICENSE-12345".to_string(),
        client_id: "CLIENT-67890".to_string(),
        expiry_date: "2025-12-31".to_string(),
        features: vec!["feature1".to_string(), "feature2".to_string()],
        server_url: "https://yourserver.com".to_string(),
        signature: "test-signature".to_string(),
        is_active: false,
    };

    // Step 1: Activate the license
    let activation_result = license.activate().await;
    assert!(activation_result.is_ok(), "License activation should succeed");
    assert!(license.is_active, "License should be active after activation");

    // Step 2: Validate the license
    let validation_result = license.validate().await;
    assert!(validation_result.is_ok(), "License validation should succeed");

    // Step 3: Deactivate the license
    let deactivation_result = license.deactivate().await;
    assert!(deactivation_result.is_ok(), "License deactivation should succeed");
    assert!(!license.is_active, "License should not be active after deactivation");
}
