use talos::config::{get_server_url, get_heartbeat_interval, is_logging_enabled};
use talos::client::client::License;

#[test]
fn test_config_loading() {
    let license = License {
        license_id: "LICENSE-12345".to_string(),
        client_id: "CLIENT-67890".to_string(),
        expiry_date: "2025-12-31".to_string(),
        features: vec!["feature1".to_string(), "feature2".to_string()],
        server_url: "https://fallback-url.com".to_string(),
        signature: "test-signature".to_string(),
        is_active: true,
    };

    // Test server URL loading
    let server_url = get_server_url(&license);
    assert_eq!(server_url, "https://yourserver.com");

    // Test heartbeat interval loading
    let heartbeat_interval = get_heartbeat_interval();
    assert_eq!(heartbeat_interval, 60);

    // Test logging flag
    let logging_enabled = is_logging_enabled();
    assert!(logging_enabled);
}
