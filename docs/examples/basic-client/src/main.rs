//! # Basic Talos Client Example
//!
//! This example demonstrates the core license lifecycle with a realistic
//! license key entry flow - NO RECOMPILATION REQUIRED to change license keys.
//!
//! ## How License Keys Work
//!
//! 1. User purchases/receives a license key (e.g., "LIC-A1B2-C3D4-E5F6-G7H8")
//! 2. User enters the key into your app (first run dialog, config file, etc.)
//! 3. Your app calls bind() to associate the key with their hardware
//! 4. Done - the license is now active on their machine
//!
//! ## Running This Example
//!
//! 1. Start a Talos server:
//!    ```bash
//!    cargo run --bin talos_server --features admin-api
//!    ```
//!
//! 2. Create a license:
//!    ```bash
//!    curl -X POST http://127.0.0.1:8080/api/v1/licenses \
//!      -H "Content-Type: application/json" \
//!      -d '{"org_id": "example-org", "features": ["basic", "export"]}'
//!    ```
//!
//! 3. Run this example - it will prompt you for the license key:
//!    ```bash
//!    cargo run
//!    ```
//!
//!    Or provide via environment variable:
//!    ```bash
//!    LICENSE_KEY="LIC-XXXX-XXXX-XXXX" cargo run
//!    ```

use std::io::{self, Write};
use std::path::PathBuf;
use talos::client::License;
use talos::errors::LicenseResult;

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Server URL - in production, load from config file or env var
fn get_server_url() -> String {
    std::env::var("TALOS_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
}

/// Path to store the license key after first entry
fn get_license_file_path() -> PathBuf {
    // In production, use proper app data directory
    // e.g., dirs::config_dir().unwrap().join("myapp").join("license.key")
    PathBuf::from("license.key")
}

// ============================================================================
// LICENSE KEY MANAGEMENT
// ============================================================================

/// Get the license key from (in order of priority):
/// 1. Environment variable (LICENSE_KEY)
/// 2. Saved license file (from previous run)
/// 3. User prompt (first run)
fn get_license_key() -> io::Result<String> {
    // Priority 1: Environment variable
    if let Ok(key) = std::env::var("LICENSE_KEY") {
        println!("Using license key from LICENSE_KEY environment variable");
        return Ok(key.trim().to_string());
    }

    // Priority 2: Saved license file
    let license_path = get_license_file_path();
    if license_path.exists() {
        let key = std::fs::read_to_string(&license_path)?;
        let key = key.trim().to_string();
        if !key.is_empty() {
            println!("Using saved license key from {:?}", license_path);
            return Ok(key);
        }
    }

    // Priority 3: Prompt user
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║                    LICENSE KEY REQUIRED                      ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");
    println!("No license key found. Please enter your license key.");
    println!("(You can also set the LICENSE_KEY environment variable)\n");

    print!("License Key: ");
    io::stdout().flush()?;

    let mut key = String::new();
    io::stdin().read_line(&mut key)?;
    let key = key.trim().to_string();

    if key.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "License key cannot be empty",
        ));
    }

    // Save for future runs
    save_license_key(&key)?;

    Ok(key)
}

/// Save the license key for future runs
fn save_license_key(key: &str) -> io::Result<()> {
    let license_path = get_license_file_path();
    std::fs::write(&license_path, key)?;
    println!("License key saved to {:?} for future runs\n", license_path);
    Ok(())
}

/// Clear saved license key (for testing or changing licenses)
#[allow(dead_code)]
fn clear_saved_license() -> io::Result<()> {
    let license_path = get_license_file_path();
    if license_path.exists() {
        std::fs::remove_file(&license_path)?;
        println!("Cleared saved license key");
    }
    Ok(())
}

// ============================================================================
// MAIN
// ============================================================================

#[tokio::main]
async fn main() -> LicenseResult<()> {
    println!("=== Talos Basic Client Example ===\n");

    // Get license key at RUNTIME (not compiled in!)
    let license_key = match get_license_key() {
        Ok(key) => key,
        Err(e) => {
            eprintln!("Failed to get license key: {}", e);
            eprintln!("\nTo use this example:");
            eprintln!("  1. Create a license via the Admin API");
            eprintln!("  2. Run again and enter the license key when prompted");
            eprintln!("  3. Or set LICENSE_KEY environment variable");
            std::process::exit(1);
        }
    };

    let server_url = get_server_url();
    println!("Server URL: {}", server_url);
    println!("License Key: {}...", &license_key[..license_key.len().min(12)]);
    println!();

    // Create a license instance with runtime values
    let mut license = License::new(license_key, server_url);

    // Step 1: Bind the license to this machine
    println!("Step 1: Binding license to this machine...");
    match license
        .bind(Some("Example App"), Some("Basic client example"))
        .await
    {
        Ok(result) => {
            println!("  ✓ License bound successfully!");
            println!("    License ID: {}", result.license_id);
            println!("    Features: {:?}", result.features);
            if let Some(tier) = &result.tier {
                println!("    Tier: {}", tier);
            }
            if let Some(expires) = &result.expires_at {
                println!("    Expires: {}", expires);
            }
        }
        Err(e) => {
            eprintln!("  ✗ Failed to bind license: {}", e);
            eprintln!("\n  Possible causes:");
            eprintln!("    - License key is incorrect");
            eprintln!("    - License is already bound to another machine");
            eprintln!("    - Server is not reachable");
            eprintln!("\n  To try a different key, delete {:?} and run again", get_license_file_path());
            return Err(e);
        }
    }
    println!();

    // Step 2: Validate the license
    println!("Step 2: Validating license...");
    match license.validate().await {
        Ok(result) => {
            println!("  ✓ License is valid!");
            println!("    Features: {:?}", result.features);

            if let Some(warning) = &result.warning {
                println!("    ⚠ Warning: {}", warning);
            }
        }
        Err(e) => {
            eprintln!("  ✗ Validation failed: {}", e);
        }
    }
    println!();

    // Step 3: Check specific features
    println!("Step 3: Checking features...");

    let features_to_check = ["basic", "export", "premium", "analytics"];

    for feature in &features_to_check {
        let result = license.validate().await?;
        let has_feature = result.has_feature(feature);
        let status = if has_feature {
            "✓ Enabled"
        } else {
            "✗ Disabled"
        };
        println!("    {} - {}", feature, status);
    }
    println!();

    // Step 4: Send a heartbeat
    println!("Step 4: Sending heartbeat...");
    match license.heartbeat().await {
        Ok(result) => {
            println!("  ✓ Heartbeat sent!");
            println!("    Server time: {}", result.server_time);
            if let Some(grace_end) = &result.grace_period_ends_at {
                println!("    Grace period ends: {}", grace_end);
            }
        }
        Err(e) => {
            eprintln!("  ✗ Heartbeat failed: {}", e);
        }
    }
    println!();

    // Step 5: Demonstrate offline validation
    println!("Step 5: Testing offline validation...");
    match license.validate_offline() {
        Ok(result) => {
            println!("  ✓ Offline validation successful!");
            println!("    Features: {:?}", result.features);
            if let Some(warning) = &result.warning {
                println!("    ⚠ Warning: {}", warning);
            }
        }
        Err(e) => {
            eprintln!("  ✗ Offline validation failed: {}", e);
            eprintln!("    (This is expected if you haven't validated recently)");
        }
    }
    println!();

    // Step 6: Release the license
    println!("Step 6: Releasing license...");
    match license.release().await {
        Ok(()) => {
            println!("  ✓ License released successfully!");
            println!("    The license can now be used on another machine.");
        }
        Err(e) => {
            eprintln!("  ✗ Failed to release license: {}", e);
        }
    }
    println!();

    println!("=== Example Complete ===");

    Ok(())
}
