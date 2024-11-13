use std::net::TcpListener;
use std::io::{Read, Write};
use serde::{Deserialize, Serialize};
use serde_json::json;

use talos::config;
use talos::key_generation;
use talos::encrypted_storage;
use talos::server::server_sim::{activate_license, deactivate_license, is_license_active};

#[derive(Debug, Deserialize, Serialize)]
struct LicenseRequest {
    license_id: String,
    client_id: String,
}

fn main() {
    println!("Starting Talos Server...");

    // Step 1: Initialize the server on port 7878
    let listener = TcpListener::bind("127.0.0.1:7878").expect("Failed to bind to port 7878");
    println!("Server running on http://127.0.0.1:7878");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buffer = [0; 512];
                stream.read(&mut buffer).expect("Failed to read request");

                // Step 2: Handle the request
                let request: LicenseRequest = match serde_json::from_slice(&buffer) {
                    Ok(req) => req,
                    Err(_) => {
                        let response = json!({ "status": "error", "message": "Invalid request format" });
                        stream.write_all(response.to_string().as_bytes()).expect("Failed to write response");
                        continue;
                    }
                };

                // Step 3: Determine action based on request type
                if buffer.starts_with(b"POST /activate") {
                    handle_activate(&mut stream, &request);
                } else if buffer.starts_with(b"POST /deactivate") {
                    handle_deactivate(&mut stream, &request);
                } else if buffer.starts_with(b"POST /validate") {
                    handle_validate(&mut stream, &request);
                } else {
                    let response = json!({ "status": "error", "message": "Unknown endpoint" });
                    stream.write_all(response.to_string().as_bytes()).expect("Failed to write response");
                }
            }
            Err(e) => eprintln!("Failed to handle connection: {}", e),
        }
    }
}

/// Handle license activation
fn handle_activate(stream: &mut impl Write, request: &LicenseRequest) {
    let success = activate_license(&request.license_id, &request.client_id);
    let response = if success {
        json!({ "status": "success", "message": "License activated" })
    } else {
        json!({ "status": "error", "message": "License activation failed" })
    };
    stream.write_all(response.to_string().as_bytes()).expect("Failed to write response");
}

/// Handle license deactivation
fn handle_deactivate(stream: &mut impl Write, request: &LicenseRequest) {
    let success = deactivate_license(&request.license_id, &request.client_id);
    let response = if success {
        json!({ "status": "success", "message": "License deactivated" })
    } else {
        json!({ "status": "error", "message": "License deactivation failed" })
    };
    stream.write_all(response.to_string().as_bytes()).expect("Failed to write response");
}

/// Handle license validation
fn handle_validate(stream: &mut impl Write, request: &LicenseRequest) {
    let is_active = is_license_active(&request.license_id, &request.client_id);
    let response = if is_active {
        json!({ "status": "success", "message": "License is valid" })
    } else {
        json!({ "status": "error", "message": "License is invalid or inactive" })
    };
    stream.write_all(response.to_string().as_bytes()).expect("Failed to write response");
}
