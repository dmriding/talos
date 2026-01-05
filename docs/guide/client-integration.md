# Client Integration Guide

This guide covers everything you need to integrate Talos licensing into your Rust application. You'll learn how to bind licenses, validate them online and offline, handle errors gracefully, and implement proper lifecycle management.

## Table of Contents

- [Adding Talos to Your Project](#adding-talos-to-your-project)
- [The License Struct](#the-license-struct)
- [Binding a License](#binding-a-license)
- [Validating a License](#validating-a-license)
- [Offline Validation](#offline-validation)
- [Feature Gating](#feature-gating)
- [Heartbeat Integration](#heartbeat-integration)
- [Releasing a License](#releasing-a-license)
- [Error Handling](#error-handling)
- [Complete Example](#complete-example)

---

## Adding Talos to Your Project

For client-only usage (no server components), add Talos with default features disabled:

```toml
[dependencies]
talos = { git = "https://github.com/dmriding/talos", default-features = false }
tokio = { version = "1", features = ["full"] }
```

This gives you the smallest possible binary with just the client library.

---

## The License Struct

The `License` struct is your main interface to the Talos licensing system. It manages:

- Communication with the license server
- Hardware fingerprinting (automatic)
- Encrypted cache for offline validation
- License state (bound, validated, etc.)

### Creating a License Instance

```rust
use talos::client::License;

// Create a new license instance
let mut license = License::new(
    "LIC-A1B2-C3D4-E5F6-G7H8".to_string(),  // Your license key
    "https://license.example.com".to_string(), // Server URL
);
```

### License Lifecycle

```
┌─────────────┐     bind()      ┌─────────────┐
│   Created   │ ───────────────►│    Bound    │
└─────────────┘                 └─────────────┘
                                       │
                                       │ validate() / heartbeat()
                                       ▼
                                ┌─────────────┐
                                │  Validated  │◄─────┐
                                └─────────────┘      │
                                       │             │
                                       │ (periodic)  │
                                       └─────────────┘
                                       │
                                       │ release()
                                       ▼
                                ┌─────────────┐
                                │  Released   │
                                └─────────────┘
```

---

## Binding a License

Binding associates a license with a specific machine using hardware fingerprinting. A license can only be bound to one machine at a time.

### Basic Binding

```rust
use talos::client::License;

async fn bind_license() -> Result<(), Box<dyn std::error::Error>> {
    let mut license = License::new(
        "LIC-A1B2-C3D4-E5F6-G7H8".to_string(),
        "https://license.example.com".to_string(),
    );

    // Bind with optional device info
    let result = license.bind(
        Some("John's Workstation"),  // Device name (shown in admin UI)
        Some("Windows 11, 32GB RAM"), // Device info (for your reference)
    ).await?;

    println!("Successfully bound to license: {}", result.license_id);
    println!("Features enabled: {:?}", result.features);
    println!("Expires: {:?}", result.expires_at);

    Ok(())
}
```

### BindResult Fields

```rust
pub struct BindResult {
    pub license_id: String,           // UUID of the license
    pub features: Vec<String>,        // Enabled features
    pub tier: Option<String>,         // License tier (e.g., "pro", "enterprise")
    pub expires_at: Option<String>,   // Expiration date (ISO 8601)
}

// Helper method
result.has_feature("premium_export")  // Returns bool
```

### What Happens During Binding

1. Talos generates a hardware fingerprint (CPU + motherboard hash)
2. Sends bind request to server with license key and hardware ID
3. Server verifies license is valid and not bound elsewhere
4. Server records the binding and returns license details
5. Client caches the validation data locally (encrypted)

---

## Validating a License

Validation checks that a license is still valid (not expired, revoked, or bound to a different machine).

### Online Validation

```rust
async fn validate_license(license: &mut License) -> Result<(), Box<dyn std::error::Error>> {
    let result = license.validate().await?;

    println!("License is valid!");
    println!("Features: {:?}", result.features);
    println!("Tier: {:?}", result.tier);

    // Check for warnings (e.g., expiring soon)
    if let Some(warning) = &result.warning {
        eprintln!("Warning: {}", warning);
    }

    Ok(())
}
```

### ValidationResult Fields

```rust
pub struct ValidationResult {
    pub features: Vec<String>,              // Enabled features
    pub tier: Option<String>,               // License tier
    pub expires_at: Option<String>,         // Expiration date
    pub grace_period_ends_at: Option<String>, // For offline validation
    pub warning: Option<String>,            // Warnings (expiring soon, etc.)
}

// Helper method
result.has_feature("analytics")  // Returns bool
```

### When to Validate

- **On application startup** - Ensure license is valid before proceeding
- **Periodically during runtime** - Every 15-60 minutes is typical
- **Before critical operations** - If you need absolute certainty

```rust
// Example: Periodic validation in background
use tokio::time::{interval, Duration};

async fn periodic_validation(mut license: License) {
    let mut interval = interval(Duration::from_secs(30 * 60)); // 30 minutes

    loop {
        interval.tick().await;

        match license.validate().await {
            Ok(result) => {
                if let Some(warning) = result.warning {
                    eprintln!("License warning: {}", warning);
                }
            }
            Err(e) => {
                eprintln!("License validation failed: {}", e);
                // Handle gracefully - maybe allow limited functionality
            }
        }
    }
}
```

---

## Offline Validation

For air-gapped systems or when the network is unavailable, Talos supports offline validation using encrypted cached data.

### How Offline Validation Works

1. Each successful `validate()` or `heartbeat()` caches license data locally
2. The cache is encrypted with AES-256-GCM using a hardware-bound key
3. The server provides a grace period (how long offline validation is allowed)
4. `validate_offline()` checks the cached data against the grace period

### Using Offline Validation

```rust
async fn check_license_offline(license: &mut License) -> bool {
    match license.validate_offline().await {
        Ok(result) => {
            println!("Offline validation successful");
            println!("Features: {:?}", result.features);

            // Check for warnings
            if let Some(warning) = &result.warning {
                // "Grace period expires in X hours" type messages
                eprintln!("Warning: {}", warning);
            }

            true
        }
        Err(e) => {
            eprintln!("Offline validation failed: {}", e);
            false
        }
    }
}
```

### Validate with Fallback

The most common pattern is to try online validation first, then fall back to offline:

```rust
async fn validate_with_fallback(license: &mut License) -> Result<bool, Box<dyn std::error::Error>> {
    match license.validate_with_fallback().await {
        Ok(result) => {
            // Check if we're in offline mode
            if let Some(warning) = &result.warning {
                if warning.contains("offline") || warning.contains("grace period") {
                    println!("Running in offline mode: {}", warning);
                }
            }

            Ok(true)
        }
        Err(e) => {
            eprintln!("All validation methods failed: {}", e);
            Ok(false)
        }
    }
}
```

### Grace Period Configuration

The grace period is set server-side when a license is suspended or configured. Typical values:

- **Development licenses**: 24 hours
- **Standard licenses**: 7 days
- **Enterprise licenses**: 30 days

---

## Feature Gating

Use feature gating to enable/disable functionality based on the license tier.

### Check Features from Validation Result

```rust
async fn check_features(license: &mut License) -> Result<(), Box<dyn std::error::Error>> {
    let result = license.validate().await?;

    // Check features
    if result.has_feature("premium_export") {
        enable_premium_export();
    }

    if result.has_feature("analytics") {
        enable_analytics();
    }

    if result.has_feature("api_access") {
        enable_api();
    }

    Ok(())
}
```

### Server-Side Feature Validation

For sensitive features, validate directly with the server:

```rust
async fn validate_feature(license: &mut License, feature: &str) -> bool {
    match license.validate_feature(feature).await {
        Ok(result) => {
            if result.allowed {
                println!("Feature '{}' is allowed", feature);
                true
            } else {
                println!("Feature '{}' not included: {:?}", feature, result.message);
                false
            }
        }
        Err(e) => {
            // Feature not in license tier returns an error
            eprintln!("Feature validation failed: {}", e);
            false
        }
    }
}
```

### Pattern: Feature-Gated Functions

```rust
use talos::client::License;

pub struct App {
    license: License,
    features_cache: Vec<String>,
}

impl App {
    pub async fn init(license_key: String, server_url: String) -> Result<Self, Box<dyn std::error::Error>> {
        let mut license = License::new(license_key, server_url);

        // Bind and cache features
        let bind_result = license.bind(None, None).await?;

        Ok(Self {
            license,
            features_cache: bind_result.features,
        })
    }

    pub fn has_feature(&self, feature: &str) -> bool {
        self.features_cache.contains(&feature.to_string())
    }

    pub fn export_data(&self) -> Result<Vec<u8>, &'static str> {
        if !self.has_feature("export") {
            return Err("Export feature not included in your license");
        }

        // Do the export...
        Ok(vec![])
    }

    pub fn advanced_analytics(&self) -> Result<(), &'static str> {
        if !self.has_feature("analytics") {
            return Err("Analytics feature requires Pro license");
        }

        // Run analytics...
        Ok(())
    }
}
```

---

## Heartbeat Integration

Heartbeats keep the license active and update the grace period for offline validation.

### Basic Heartbeat

```rust
async fn send_heartbeat(license: &mut License) -> Result<(), Box<dyn std::error::Error>> {
    let result = license.heartbeat().await?;

    println!("Heartbeat successful");
    println!("Server time: {}", result.server_time);

    if let Some(grace_end) = result.grace_period_ends_at {
        println!("Grace period ends: {}", grace_end);
    }

    Ok(())
}
```

### Background Heartbeat Task

```rust
use tokio::time::{interval, Duration};
use std::sync::Arc;
use tokio::sync::Mutex;

async fn heartbeat_task(license: Arc<Mutex<License>>) {
    // Send heartbeat every 5 minutes
    let mut interval = interval(Duration::from_secs(5 * 60));

    loop {
        interval.tick().await;

        let mut license = license.lock().await;
        match license.heartbeat().await {
            Ok(_) => {
                // Heartbeat successful - grace period extended
            }
            Err(e) => {
                eprintln!("Heartbeat failed: {} (will retry)", e);
                // Don't panic - the app can continue with cached validation
            }
        }
    }
}

// Usage
#[tokio::main]
async fn main() {
    let license = Arc::new(Mutex::new(License::new(
        "LIC-XXXX".to_string(),
        "https://license.example.com".to_string(),
    )));

    // Spawn heartbeat task
    let license_clone = Arc::clone(&license);
    tokio::spawn(heartbeat_task(license_clone));

    // Your app logic here...
}
```

---

## Releasing a License

When your application closes, release the license so it can be used on another machine.

### Basic Release

```rust
async fn shutdown(license: &mut License) -> Result<(), Box<dyn std::error::Error>> {
    license.release().await?;
    println!("License released successfully");
    Ok(())
}
```

### Graceful Shutdown Pattern

```rust
use tokio::signal;

async fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    let mut license = License::new(
        "LIC-XXXX".to_string(),
        "https://license.example.com".to_string(),
    );

    // Bind on startup
    license.bind(None, None).await?;

    // Set up graceful shutdown
    let shutdown = async {
        signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
        println!("\nShutting down...");
    };

    // Run your app until shutdown signal
    tokio::select! {
        _ = your_app_logic() => {}
        _ = shutdown => {}
    }

    // Always release on shutdown
    if let Err(e) = license.release().await {
        eprintln!("Warning: Failed to release license: {}", e);
        // Not fatal - the admin can manually release it
    }

    Ok(())
}
```

---

## Error Handling

Talos provides typed errors for precise error handling.

### ClientApiError

```rust
use talos::client::errors::{ClientApiError, ClientErrorCode};

async fn handle_errors(license: &mut License) {
    match license.validate().await {
        Ok(result) => {
            println!("Valid: {:?}", result.features);
        }
        Err(e) => {
            // Check if it's an API error
            if let Some(api_error) = e.as_api_error() {
                match api_error.code {
                    ClientErrorCode::LicenseNotFound => {
                        println!("License key is invalid");
                    }
                    ClientErrorCode::LicenseExpired => {
                        println!("License has expired - please renew");
                    }
                    ClientErrorCode::HardwareMismatch => {
                        println!("License is bound to a different machine");
                    }
                    ClientErrorCode::LicenseRevoked => {
                        println!("License has been revoked");
                    }
                    ClientErrorCode::NotBound => {
                        println!("License is not bound - call bind() first");
                    }
                    _ => {
                        println!("License error: {}", api_error.message);
                    }
                }
            } else {
                // Network error or other issue
                println!("Connection error: {}", e);
            }
        }
    }
}
```

### Error Code Reference

| Code | Description | Action |
|------|-------------|--------|
| `LicenseNotFound` | License key doesn't exist | Check key is correct |
| `LicenseExpired` | License has expired | Prompt user to renew |
| `LicenseRevoked` | License was revoked by admin | Contact support |
| `LicenseSuspended` | Temporarily suspended | May have grace period |
| `LicenseBlacklisted` | Permanently banned | Contact support |
| `HardwareMismatch` | Different machine | Release from other machine |
| `AlreadyBound` | Already bound elsewhere | Release first |
| `NotBound` | Not bound to any machine | Call `bind()` first |
| `FeatureNotIncluded` | Feature not in tier | Upgrade license |
| `NetworkError` | Connection failed | Check network, retry |

### Retry Strategy

```rust
use tokio::time::{sleep, Duration};

async fn validate_with_retry(license: &mut License, max_retries: u32) -> Result<(), Box<dyn std::error::Error>> {
    let mut retries = 0;

    loop {
        match license.validate().await {
            Ok(_) => return Ok(()),
            Err(e) => {
                // Don't retry for permanent errors
                if let Some(api_error) = e.as_api_error() {
                    match api_error.code {
                        ClientErrorCode::LicenseNotFound |
                        ClientErrorCode::LicenseRevoked |
                        ClientErrorCode::LicenseBlacklisted => {
                            return Err(e.into());
                        }
                        _ => {}
                    }
                }

                retries += 1;
                if retries >= max_retries {
                    return Err(e.into());
                }

                // Exponential backoff
                let delay = Duration::from_secs(2u64.pow(retries));
                eprintln!("Validation failed, retrying in {:?}...", delay);
                sleep(delay).await;
            }
        }
    }
}
```

---

## Complete Example

Here's a complete example showing all the concepts together:

```rust
use talos::client::License;
use talos::client::errors::ClientErrorCode;
use tokio::time::{interval, Duration};
use tokio::signal;
use std::sync::Arc;
use tokio::sync::Mutex;

struct LicensedApp {
    license: Arc<Mutex<License>>,
    features: Vec<String>,
}

impl LicensedApp {
    async fn new(license_key: &str, server_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut license = License::new(license_key.to_string(), server_url.to_string());

        // Try to bind
        let bind_result = license.bind(
            Some(&hostname::get()?.to_string_lossy()),
            Some(&std::env::consts::OS),
        ).await?;

        println!("License activated successfully!");
        println!("Features: {:?}", bind_result.features);

        let features = bind_result.features.clone();
        let license = Arc::new(Mutex::new(license));

        Ok(Self { license, features })
    }

    fn has_feature(&self, feature: &str) -> bool {
        self.features.contains(&feature.to_string())
    }

    async fn start_heartbeat(&self) {
        let license = Arc::clone(&self.license);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5 * 60));

            loop {
                interval.tick().await;

                let mut license = license.lock().await;
                if let Err(e) = license.heartbeat().await {
                    eprintln!("Heartbeat failed: {}", e);
                }
            }
        });
    }

    async fn validate(&self) -> bool {
        let mut license = self.license.lock().await;

        // Try online first, then offline
        match license.validate_with_fallback().await {
            Ok(result) => {
                if let Some(warning) = result.warning {
                    eprintln!("License warning: {}", warning);
                }
                true
            }
            Err(e) => {
                eprintln!("License validation failed: {}", e);
                false
            }
        }
    }

    async fn shutdown(&self) {
        let mut license = self.license.lock().await;

        if let Err(e) = license.release().await {
            eprintln!("Failed to release license: {}", e);
        } else {
            println!("License released");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize
    let app = LicensedApp::new(
        "LIC-A1B2-C3D4-E5F6-G7H8",
        "https://license.example.com",
    ).await?;

    // Start background heartbeat
    app.start_heartbeat().await;

    // Check features
    if app.has_feature("premium") {
        println!("Premium features enabled!");
    }

    // Periodic validation
    let app_clone = app.clone(); // Would need Clone impl
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30 * 60));
        loop {
            interval.tick().await;
            if !app_clone.validate().await {
                eprintln!("License validation failed - some features may be disabled");
            }
        }
    });

    // Wait for shutdown signal
    signal::ctrl_c().await?;
    println!("\nShutting down...");

    // Clean up
    app.shutdown().await;

    Ok(())
}
```

---

## Next Steps

- **[Server Deployment Guide](server-deployment.md)** - Deploy your own license server
- **[Admin API Guide](admin-api.md)** - Manage licenses programmatically
- **[Troubleshooting](troubleshooting.md)** - Common issues and solutions
