use talos::client::License;
use std::fs;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let license_json = fs::read_to_string("test_license.json")?;
    let license: License = serde_json::from_str(&license_json)?;

    if license.validate().await? {
        println!("License is valid.");
    } else {
        println!("License is invalid.");
    }

    Ok(())
}
