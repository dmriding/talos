use talos::client::License;
use std::fs;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let license_json = fs::read_to_string("test_license.json")?;
    let mut license: License = serde_json::from_str(&license_json)?;

    // Activate the license
    license.activate().await?;
    println!("License activated.");

    // Validate the license
    if license.validate().await? {
        println!("License is valid.");
    } else {
        println!("License is invalid.");
    }

    // Deactivate the license
    license.deactivate().await?;
    println!("License deactivated.");

    Ok(())
}
