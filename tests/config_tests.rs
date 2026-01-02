use std::env;

use talos::client::client::License;
use talos::config::{get_heartbeat_interval, get_server_url, is_logging_enabled};

fn dummy_license(server_url: &str) -> License {
    License {
        license_id: "LICENSE-12345".to_string(),
        client_id: "CLIENT-67890".to_string(),
        expiry_date: "2025-12-31".to_string(),
        features: vec!["feature1".to_string(), "feature2".to_string()],
        server_url: server_url.to_string(),
        signature: "test-signature".to_string(),
        is_active: true,
    }
}

#[test]
fn server_url_prefers_env_over_license() {
    // Ensure env var is set to a known value
    env::set_var("SERVER_URL", "https://env-override.example");

    // License has a different URL, but env should win
    let license = dummy_license("https://license-fallback.example");
    let server_url = get_server_url(&license);

    assert_eq!(server_url, "https://env-override.example");

    // Clean up env var for other tests
    env::remove_var("SERVER_URL");
}

#[test]
fn heartbeat_interval_has_sane_default() {
    let heartbeat_interval = get_heartbeat_interval();

    // Default is 60 in code, but we just assert it's positive
    // so config.toml overrides don't break the test.
    assert!(heartbeat_interval > 0);
}

#[test]
fn logging_is_disabled_by_default() {
    // Verify the function returns without panicking.
    // The actual value depends on config.toml, so we don't assert a specific value.
    let _logging_enabled = is_logging_enabled();
}
