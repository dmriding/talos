use talos::client::client::License;
use talos::hardware::get_hardware_id;

#[tokio::main]
async fn main() {
    let server_url = std::env::var("SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
    let hw = get_hardware_id();

    let mut license = License {
        license_id: "LICENSE-TEST-001".to_string(),
        client_id: hw.clone(),
        expiry_date: "2025-12-31".into(),
        features: vec!["featureA".into(), "featureB".into()],
        server_url,
        signature: "dummy".into(),
        is_active: false,
    };

    println!("Activating license...");
    match license.activate().await {
        Ok(_) => println!("Activation successful! is_active={}", license.is_active),
        Err(e) => println!("Activation failed: {e}"),
    }

    println!("Validating license...");
    match license.validate().await {
        Ok(valid) => println!("Validation: {valid}"),
        Err(e) => println!("Validation failed: {e}"),
    }

    println!("Heartbeat...");
    match license.heartbeat().await {
        Ok(ok) => println!("Heartbeat success={ok}"),
        Err(e) => println!("Heartbeat failed: {e}"),
    }

    println!("Deactivating...");
    match license.deactivate().await {
        Ok(_) => println!("Deactivation successful"),
        Err(e) => println!("Deactivation failed: {e}"),
    }
}
