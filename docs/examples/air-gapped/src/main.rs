//! # Air-Gapped/Offline Validation Example
//!
//! This example demonstrates how to use Talos in environments where network
//! connectivity is intermittent or unavailable (air-gapped systems).
//!
//! ## Key Concepts
//!
//! 1. **Encrypted Cache**: Each successful validation caches license data locally,
//!    encrypted with AES-256-GCM using a hardware-bound key.
//!
//! 2. **Grace Period**: The server provides a grace period during which offline
//!    validation is allowed. This is typically 7-30 days.
//!
//! 3. **Fallback Validation**: `validate_with_fallback()` tries online first,
//!    then falls back to cached offline validation.
//!
//! ## Use Cases
//!
//! - Industrial systems without internet access
//! - Field deployments with intermittent connectivity
//! - High-security environments with restricted networking
//! - Desktop applications that should work offline
//!
//! ## Running This Example
//!
//! ```bash
//! # Step 1: Run with server online to cache the license
//! LICENSE_KEY="LIC-XXXX-XXXX-XXXX" cargo run
//!
//! # Step 2: Stop the server, then run in offline mode
//! cargo run -- --offline
//! ```

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
    std::fs::write(&license_path, &key)?;
    println!("License key saved to {:?} for future runs\n", license_path);

    Ok(key)
}

// ============================================================================
// MAIN
// ============================================================================

#[tokio::main]
async fn main() -> LicenseResult<()> {
    // Check for --offline flag
    let args: Vec<String> = std::env::args().collect();
    let offline_mode = args.iter().any(|a| a == "--offline" || a == "-o");

    if offline_mode {
        println!("=== Talos Air-Gapped Example (OFFLINE MODE) ===\n");
        println!("Running in offline mode - server connection not required.\n");
        return run_offline_only().await;
    }

    println!("=== Talos Air-Gapped Example ===\n");
    println!("This will connect to the server to cache license data.");
    println!("After running, use '--offline' flag to test offline mode:\n");
    println!("  cargo run -- --offline\n");

    run_full_demo().await
}

/// Run the full demo with online activation first
async fn run_full_demo() -> LicenseResult<()> {
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

    let mut license = License::new(license_key, server_url);

    // ========================================================================
    // PHASE 1: Initial Online Activation
    // ========================================================================
    // This should happen when the system has network access (e.g., during
    // initial deployment or periodic maintenance windows).

    println!("Phase 1: Initial Online Activation");
    println!("-----------------------------------");

    // Bind the license (requires network)
    println!("Binding license...");
    match license
        .bind(Some("Air-Gapped System"), Some("Industrial Controller"))
        .await
    {
        Ok(result) => {
            println!("  ✓ License bound!");
            println!("    Features: {:?}", result.features);
        }
        Err(e) => {
            eprintln!("  ✗ Bind failed: {}", e);
            eprintln!("\n  For air-gapped systems, you must bind at least once");
            eprintln!("  when network is available.");
            return Err(e);
        }
    }

    // Validate to populate the cache
    println!("Validating and caching license data...");
    match license.validate().await {
        Ok(result) => {
            println!("  ✓ License validated and cached!");
            println!("    Features: {:?}", result.features);
            if let Some(grace_end) = &result.grace_period_ends_at {
                println!("    Grace period until: {}", grace_end);
            }
        }
        Err(e) => {
            eprintln!("  ✗ Validation failed: {}", e);
            return Err(e);
        }
    }

    // Send heartbeat to extend grace period
    println!("Extending grace period with heartbeat...");
    match license.heartbeat().await {
        Ok(result) => {
            println!("  ✓ Grace period extended!");
            if let Some(grace_end) = &result.grace_period_ends_at {
                println!("    New grace period until: {}", grace_end);
            }
        }
        Err(e) => {
            eprintln!("  ✗ Heartbeat failed: {}", e);
        }
    }

    println!();

    // ========================================================================
    // PHASE 2: Offline Operation (simulated)
    // ========================================================================

    println!("Phase 2: Offline Operation (Simulated)");
    println!("--------------------------------------");
    println!("Testing offline validation with cached data...\n");

    run_offline_validation(&license)?;

    println!();

    // ========================================================================
    // BEST PRACTICES SUMMARY
    // ========================================================================

    print_best_practices();

    // NOTE: We do NOT release the license here so the cache persists
    // for testing offline mode. In production, you would typically
    // keep the license bound for air-gapped systems.

    println!("=== Example Complete ===");
    println!("\nThe license cache has been saved to disk.");
    println!("Now stop the server and run:");
    println!("\n  cargo run -- --offline\n");
    println!("This will demonstrate true offline validation!");

    Ok(())
}

/// Run in offline-only mode (no server connection)
async fn run_offline_only() -> LicenseResult<()> {
    // Try to load cached license state from disk
    println!("Loading cached license data from disk...");
    let license = match License::load_from_disk().await {
        Ok(lic) => {
            println!("  ✓ Cached license data loaded!");
            println!("    License Key: {}...", &lic.license_key[..lic.license_key.len().min(12)]);
            println!();
            lic
        }
        Err(e) => {
            eprintln!("  ✗ Failed to load cached data: {}", e);
            eprintln!("\n  You must run this example once WITH the server");
            eprintln!("  to populate the cache before offline mode works.");
            eprintln!("\n  Run without --offline first:");
            eprintln!("    cargo run");
            std::process::exit(1);
        }
    };

    // ========================================================================
    // OFFLINE VALIDATION
    // ========================================================================

    println!("=== Offline Validation Demo ===");
    println!("(Server is not being contacted)\n");

    run_offline_validation(&license)?;

    println!();
    print_best_practices();

    println!("\n=== Offline Demo Complete ===");
    println!("\nThe license was validated entirely from the encrypted local cache.");
    println!("No network connection was required!");

    Ok(())
}

/// Run offline validation and feature checks
fn run_offline_validation(license: &License) -> LicenseResult<()> {
    // Method 1: Direct offline validation
    println!("Method 1: Direct offline validation");
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
            eprintln!("    The grace period may have expired.");
            eprintln!("    Connect to network and re-validate.");
            return Err(e);
        }
    }

    println!();

    // Feature gating check
    println!("Feature Gating (from cache):");
    let validation = license.validate_offline()?;

    let features = ["basic", "export", "advanced", "premium"];
    for feature in features {
        let enabled = validation.has_feature(feature);
        let status = if enabled { "✓" } else { "✗" };
        println!("  {} {}", status, feature);
    }

    println!();

    // Grace period status
    println!("Grace Period Status:");
    if let Some(grace_end) = &validation.grace_period_ends_at {
        println!("  Expires: {}", grace_end);
        println!("  Status: Valid for offline use");
    } else {
        println!("  No grace period set");
    }

    Ok(())
}

/// Print best practices summary
fn print_best_practices() {
    println!("=== Best Practices for Air-Gapped Systems ===");
    println!();
    println!("1. INITIAL ACTIVATION");
    println!("   - Always bind() and validate() when network is available");
    println!("   - This populates the encrypted cache");
    println!();
    println!("2. PERIODIC REFRESH (during maintenance windows)");
    println!("   - Call heartbeat() to extend the grace period");
    println!("   - Call validate() to refresh cached features");
    println!();
    println!("3. DAILY OPERATION");
    println!("   - Use validate_with_fallback() for automatic handling");
    println!("   - Or use validate_offline() if you know you're offline");
    println!();
    println!("4. GRACE PERIOD MONITORING");
    println!("   - Check warnings from validation results");
    println!("   - Schedule network access before grace period expires");
    println!();
    println!("5. CACHE SECURITY");
    println!("   - Cache is encrypted with hardware-bound key");
    println!("   - Cannot be copied to another machine");
    println!("   - Tamper-evident via GCM authentication");
    println!();
}
