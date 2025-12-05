// src/client/main.rs

use talos::client::key_generation::generate_secure_key;
use talos::errors::LicenseResult;

/// Simple demo entrypoint for the Talos client.
///
/// Right now this just generates a secure key based on:
/// - hardware ID,
/// - timestamp,
/// - device-specific private key (generated + stored if needed).
///
/// The main licensing flows (activate/validate/deactivate/heartbeat)
/// are exposed via the `License` type in `talos::client::client` and
/// are intended to be called from your actual application or from
/// more advanced CLI tooling later.
fn main() -> LicenseResult<()> {
    let secure_key = generate_secure_key()?;
    println!("Generated secure key: {}", secure_key);

    Ok(())
}
