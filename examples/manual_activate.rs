use talos::client::License;
use talos::hardware::get_hardware_id;

#[tokio::main]
async fn main() {
    let server_url = std::env::var("SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
    let hw = get_hardware_id();

    // Create license using the new constructor
    let mut license = License::new("LICENSE-TEST-001".to_string(), server_url);
    license.license_id = "LICENSE-TEST-001".to_string();
    license.client_id = hw.clone();
    license.expiry_date = "2025-12-31".into();
    license.features = vec!["featureA".into(), "featureB".into()];
    license.signature = "dummy".into();
    license.is_active = false;

    // Using legacy activate method (deprecated, use bind() for new code)
    println!("Activating license...");
    #[allow(deprecated)]
    match license.activate().await {
        Ok(_) => println!("Activation successful! is_active={}", license.is_active),
        Err(e) => println!("Activation failed: {e}"),
    }

    println!("Validating license...");
    match license.validate().await {
        Ok(result) => println!("Validation successful: features={:?}", result.features),
        Err(e) => println!("Validation failed: {e}"),
    }

    println!("Heartbeat...");
    match license.heartbeat().await {
        Ok(result) => println!("Heartbeat success, server_time={}", result.server_time),
        Err(e) => println!("Heartbeat failed: {e}"),
    }

    // Using legacy deactivate method (deprecated, use release() for new code)
    println!("Deactivating...");
    #[allow(deprecated)]
    match license.deactivate().await {
        Ok(_) => println!("Deactivation successful"),
        Err(e) => println!("Deactivation failed: {e}"),
    }
}
