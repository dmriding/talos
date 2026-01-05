use std::error::Error;
use std::fs;
use talos::client::License;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load the test license from a JSON file
    let license_json = fs::read_to_string("test_license.json")?;
    let mut license: License = serde_json::from_str(&license_json)?;

    // Activate the license (legacy method, deprecated - use bind() for new code)
    #[allow(deprecated)]
    license.activate().await?;
    println!("License activated.");

    // Validate the license
    let result = license.validate().await?;
    println!("License is valid. Features: {:?}", result.features);

    // Deactivate the license (legacy method, deprecated - use release() for new code)
    #[allow(deprecated)]
    license.deactivate().await?;
    println!("License deactivated.");

    Ok(())
}
