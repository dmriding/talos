use std::fs;
use talos::client::client::License; // Adjusted import path
use talos::config;

fn main() {
    // Load the test license from the JSON file
    let license_json =
        fs::read_to_string("test_license.json").expect("Failed to read license file");
    let license: License = serde_json::from_str(&license_json).expect("Failed to parse license");

    // Get the server URL using the config module
    let server_url = config::get_server_url(&license);
    println!("Server URL: {}", server_url);
}
