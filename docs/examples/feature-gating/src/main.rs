//! # Feature Gating Example
//!
//! This example demonstrates how to use Talos to gate application features
//! based on the user's license. This is essential for:
//!
//! - **Tiered pricing**: Basic vs Pro vs Enterprise features
//! - **Add-on modules**: Export, Analytics, Integrations, etc.
//! - **Trial limitations**: Limited features during trial period
//!
//! ## Running This Example
//!
//! ```bash
//! # Create a license with specific features, then:
//! LICENSE_KEY="LIC-XXXX-XXXX-XXXX" cargo run
//! ```

use std::io::{self, Write};
use std::path::PathBuf;
use talos::client::License;
use talos::errors::LicenseResult;

// ============================================================================
// SIMULATED APPLICATION FEATURES
// ============================================================================

/// Represents features that can be enabled/disabled by license
#[derive(Debug, Clone, Copy, PartialEq)]
enum Feature {
    /// Basic functionality - always available
    Basic,
    /// Export data to CSV/JSON
    Export,
    /// Advanced analytics and reporting
    Analytics,
    /// Premium support features
    Premium,
    /// API access for integrations
    Api,
    /// White-label/custom branding
    Whitelabel,
}

impl Feature {
    fn name(&self) -> &'static str {
        match self {
            Feature::Basic => "basic",
            Feature::Export => "export",
            Feature::Analytics => "analytics",
            Feature::Premium => "premium",
            Feature::Api => "api",
            Feature::Whitelabel => "whitelabel",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Feature::Basic => "Core application functionality",
            Feature::Export => "Export data to CSV, JSON, Excel",
            Feature::Analytics => "Advanced analytics and reporting dashboards",
            Feature::Premium => "Priority support and extended features",
            Feature::Api => "REST API access for integrations",
            Feature::Whitelabel => "Custom branding and white-label options",
        }
    }
}

// ============================================================================
// FEATURE-GATED FUNCTIONS
// ============================================================================

/// A feature-gated function that requires the "export" feature
fn export_data(license: &License, format: &str) -> Result<String, String> {
    // Check if the license has the export feature
    if !license_has_feature(license, Feature::Export) {
        return Err(format!(
            "Export feature not available. Upgrade your license to access {} export.",
            format
        ));
    }

    // Feature is enabled - perform the export
    Ok(format!(
        "Successfully exported data to {} format! (1,234 records)",
        format
    ))
}

/// A feature-gated function that requires the "analytics" feature
fn generate_report(license: &License, report_type: &str) -> Result<String, String> {
    if !license_has_feature(license, Feature::Analytics) {
        return Err(
            "Analytics feature not available. Upgrade to Pro or Enterprise for advanced reports."
                .to_string(),
        );
    }

    Ok(format!(
        "Generated {} report with 15 charts and 42 insights!",
        report_type
    ))
}

/// A feature-gated function that requires the "api" feature
fn get_api_key(license: &License) -> Result<String, String> {
    if !license_has_feature(license, Feature::Api) {
        return Err(
            "API access not available. Contact sales to add API access to your license."
                .to_string(),
        );
    }

    Ok("API Key: sk_live_abc123xyz789 (use this in your integrations)".to_string())
}

/// A feature-gated function that requires the "premium" feature
fn contact_priority_support(license: &License) -> Result<String, String> {
    if !license_has_feature(license, Feature::Premium) {
        return Err(
            "Priority support not available. Upgrade to Premium for 24/7 dedicated support."
                .to_string(),
        );
    }

    Ok("Priority support ticket created! Our team will respond within 1 hour.".to_string())
}

/// Helper to check if a license has a specific feature
fn license_has_feature(license: &License, feature: Feature) -> bool {
    license
        .cached
        .as_ref()
        .map(|c| c.has_feature(feature.name()))
        .unwrap_or(false)
}

// ============================================================================
// CONFIGURATION
// ============================================================================

fn get_server_url() -> String {
    std::env::var("TALOS_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
}

fn get_license_file_path() -> PathBuf {
    PathBuf::from("license.key")
}

fn get_license_key() -> io::Result<String> {
    // Priority 1: Environment variable
    if let Ok(key) = std::env::var("LICENSE_KEY") {
        return Ok(key.trim().to_string());
    }

    // Priority 2: Saved license file
    let license_path = get_license_file_path();
    if license_path.exists() {
        let key = std::fs::read_to_string(&license_path)?;
        let key = key.trim().to_string();
        if !key.is_empty() {
            return Ok(key);
        }
    }

    // Priority 3: Prompt user
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║                    LICENSE KEY REQUIRED                      ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");
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

    std::fs::write(&license_path, &key)?;
    Ok(key)
}

// ============================================================================
// MAIN
// ============================================================================

#[tokio::main]
async fn main() -> LicenseResult<()> {
    println!("=== Talos Feature Gating Example ===\n");

    // Get license key at runtime
    let license_key = match get_license_key() {
        Ok(key) => key,
        Err(e) => {
            eprintln!("Failed to get license key: {}", e);
            std::process::exit(1);
        }
    };

    let server_url = get_server_url();
    println!("License Key: {}...", &license_key[..license_key.len().min(12)]);
    println!("Server URL: {}\n", server_url);

    // Create and bind the license
    let mut license = License::new(license_key, server_url);

    println!("Step 1: Binding and validating license...");
    match license
        .bind(Some("Feature Demo App"), Some("Demo System"))
        .await
    {
        Ok(_) => println!("  ✓ License bound"),
        Err(e) => {
            eprintln!("  ✗ Bind failed: {}", e);
            std::process::exit(1);
        }
    }

    match license.validate().await {
        Ok(result) => {
            println!("  ✓ License validated");
            println!("  Licensed features: {:?}\n", result.features);
        }
        Err(e) => {
            eprintln!("  ✗ Validation failed: {}", e);
            std::process::exit(1);
        }
    }

    // ========================================================================
    // SHOW FEATURE STATUS
    // ========================================================================

    println!("Step 2: Checking feature availability...\n");

    let all_features = [
        Feature::Basic,
        Feature::Export,
        Feature::Analytics,
        Feature::Premium,
        Feature::Api,
        Feature::Whitelabel,
    ];

    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│                    LICENSE FEATURE STATUS                    │");
    println!("├──────────────┬──────────┬────────────────────────────────────┤");
    println!("│ Feature      │ Status   │ Description                        │");
    println!("├──────────────┼──────────┼────────────────────────────────────┤");

    for feature in &all_features {
        let enabled = license_has_feature(&license, *feature);
        let status = if enabled { "✓ ON " } else { "✗ OFF" };
        let status_color = if enabled { status } else { status };
        println!(
            "│ {:<12} │ {} │ {:<34} │",
            feature.name(),
            status_color,
            feature.description()
        );
    }

    println!("└──────────────┴──────────┴────────────────────────────────────┘\n");

    // ========================================================================
    // DEMONSTRATE FEATURE GATING
    // ========================================================================

    println!("Step 3: Attempting to use features...\n");

    // Try export (may or may not be enabled)
    println!("─── Export Feature ───");
    match export_data(&license, "CSV") {
        Ok(msg) => println!("  ✓ {}", msg),
        Err(msg) => println!("  ✗ BLOCKED: {}", msg),
    }
    println!();

    // Try analytics (may or may not be enabled)
    println!("─── Analytics Feature ───");
    match generate_report(&license, "Monthly Sales") {
        Ok(msg) => println!("  ✓ {}", msg),
        Err(msg) => println!("  ✗ BLOCKED: {}", msg),
    }
    println!();

    // Try API access (may or may not be enabled)
    println!("─── API Access Feature ───");
    match get_api_key(&license) {
        Ok(msg) => println!("  ✓ {}", msg),
        Err(msg) => println!("  ✗ BLOCKED: {}", msg),
    }
    println!();

    // Try premium support (may or may not be enabled)
    println!("─── Premium Support Feature ───");
    match contact_priority_support(&license) {
        Ok(msg) => println!("  ✓ {}", msg),
        Err(msg) => println!("  ✗ BLOCKED: {}", msg),
    }
    println!();

    // ========================================================================
    // SHOW UPGRADE PATH
    // ========================================================================

    // Count disabled features
    let disabled_count = all_features
        .iter()
        .filter(|f| !license_has_feature(&license, **f))
        .count();

    if disabled_count > 0 {
        println!("═══════════════════════════════════════════════════════════════");
        println!("                      UPGRADE YOUR LICENSE                       ");
        println!("═══════════════════════════════════════════════════════════════");
        println!();
        println!("  You have {} feature(s) that could be unlocked!", disabled_count);
        println!();
        println!("  Missing features:");
        for feature in &all_features {
            if !license_has_feature(&license, *feature) {
                println!("    • {} - {}", feature.name(), feature.description());
            }
        }
        println!();
        println!("  Contact sales@example.com to upgrade your license.");
        println!();
    }

    // ========================================================================
    // CLEANUP
    // ========================================================================

    println!("Step 4: Releasing license...");
    match license.release().await {
        Ok(_) => println!("  ✓ License released\n"),
        Err(e) => println!("  ⚠ Release failed (non-critical): {}\n", e),
    }

    println!("=== Example Complete ===");

    Ok(())
}
