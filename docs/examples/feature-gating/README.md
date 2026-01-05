# Feature Gating Example

This example demonstrates how to use Talos to gate application features based on the user's license. This is the most common use case for a licensing system:

- **Tiered pricing**: Basic vs Pro vs Enterprise features
- **Add-on modules**: Export, Analytics, Integrations, etc.
- **Trial limitations**: Limited features during trial period

## How Feature Gating Works

1. **Create licenses with specific features** via the Admin API
2. **Bind and validate** the license in your application
3. **Check features** before allowing access to premium functionality
4. **Show upgrade prompts** when users try to access locked features

## Running the Example

### 1. Start the Talos Server

```bash
# In the main Talos directory
cargo run --bin talos_server --features admin-api
```

### 2. Create a License with Limited Features

Create a "Basic" license with only `basic` and `export` features:

**PowerShell:**
```powershell
Invoke-RestMethod -Uri http://127.0.0.1:8080/api/v1/licenses -Method POST -ContentType "application/json" -Body '{"org_id": "demo-user", "features": ["basic", "export"]}'
```

**curl:**
```bash
curl -X POST http://127.0.0.1:8080/api/v1/licenses \
  -H "Content-Type: application/json" \
  -d '{"org_id": "demo-user", "features": ["basic", "export"]}'
```

Note the `license_key` from the response.

### 3. Run the Example

**PowerShell (Windows):**
```powershell
$env:LICENSE_KEY="LIC-YOUR-KEY-HERE"; cargo run
```

**bash (Mac/Linux):**
```bash
LICENSE_KEY="LIC-YOUR-KEY-HERE" cargo run
```

### 4. See Feature Blocking in Action

With a basic license (only `basic` and `export`), you'll see:

```
Step 3: Attempting to use features...

─── Export Feature ───
  ✓ Successfully exported data to CSV format! (1,234 records)

─── Analytics Feature ───
  ✗ BLOCKED: Analytics feature not available. Upgrade to Pro or Enterprise for advanced reports.

─── API Access Feature ───
  ✗ BLOCKED: API access not available. Contact sales to add API access to your license.

─── Premium Support Feature ───
  ✗ BLOCKED: Priority support not available. Upgrade to Premium for 24/7 dedicated support.
```

### 5. Try with a Full-Featured License

Create an "Enterprise" license with all features:

**PowerShell:**
```powershell
Invoke-RestMethod -Uri http://127.0.0.1:8080/api/v1/licenses -Method POST -ContentType "application/json" -Body '{"org_id": "enterprise-user", "features": ["basic", "export", "analytics", "premium", "api", "whitelabel"]}'
```

**curl:**
```bash
curl -X POST http://127.0.0.1:8080/api/v1/licenses \
  -H "Content-Type: application/json" \
  -d '{"org_id": "enterprise-user", "features": ["basic", "export", "analytics", "premium", "api", "whitelabel"]}'
```

Run with the new license key:

**PowerShell (Windows):**
```powershell
# Delete old key first
Remove-Item license.key

$env:LICENSE_KEY="LIC-NEW-KEY-HERE"; cargo run
```

**bash (Mac/Linux):**
```bash
# Delete old key first
rm license.key

LICENSE_KEY="LIC-NEW-KEY-HERE" cargo run
```

Now all features will be enabled!

## Expected Output

### Basic License (limited features)

```
=== Talos Feature Gating Example ===

License Key: LIC-A1B2-C3D4...
Server URL: http://127.0.0.1:8080

Step 1: Binding and validating license...
  ✓ License bound
  ✓ License validated
  Licensed features: ["basic", "export"]

Step 2: Checking feature availability...

┌─────────────────────────────────────────────────────────────┐
│                    LICENSE FEATURE STATUS                    │
├──────────────┬──────────┬────────────────────────────────────┤
│ Feature      │ Status   │ Description                        │
├──────────────┼──────────┼────────────────────────────────────┤
│ basic        │ ✓ ON  │ Core application functionality     │
│ export       │ ✓ ON  │ Export data to CSV, JSON, Excel    │
│ analytics    │ ✗ OFF │ Advanced analytics and reporting   │
│ premium      │ ✗ OFF │ Priority support and extended      │
│ api          │ ✗ OFF │ REST API access for integrations   │
│ whitelabel   │ ✗ OFF │ Custom branding and white-label    │
└──────────────┴──────────┴────────────────────────────────────┘

Step 3: Attempting to use features...

─── Export Feature ───
  ✓ Successfully exported data to CSV format! (1,234 records)

─── Analytics Feature ───
  ✗ BLOCKED: Analytics feature not available. Upgrade to Pro or Enterprise.

─── API Access Feature ───
  ✗ BLOCKED: API access not available. Contact sales to add API access.

─── Premium Support Feature ───
  ✗ BLOCKED: Priority support not available. Upgrade to Premium for 24/7 support.

═══════════════════════════════════════════════════════════════
                      UPGRADE YOUR LICENSE
═══════════════════════════════════════════════════════════════

  You have 4 feature(s) that could be unlocked!

  Missing features:
    • analytics - Advanced analytics and reporting dashboards
    • premium - Priority support and extended features
    • api - REST API access for integrations
    • whitelabel - Custom branding and white-label options

  Contact sales@example.com to upgrade your license.

Step 4: Releasing license...
  ✓ License released

=== Example Complete ===
```

### Enterprise License (all features)

```
Step 2: Checking feature availability...

┌─────────────────────────────────────────────────────────────┐
│                    LICENSE FEATURE STATUS                    │
├──────────────┬──────────┬────────────────────────────────────┤
│ Feature      │ Status   │ Description                        │
├──────────────┼──────────┼────────────────────────────────────┤
│ basic        │ ✓ ON  │ Core application functionality     │
│ export       │ ✓ ON  │ Export data to CSV, JSON, Excel    │
│ analytics    │ ✓ ON  │ Advanced analytics and reporting   │
│ premium      │ ✓ ON  │ Priority support and extended      │
│ api          │ ✓ ON  │ REST API access for integrations   │
│ whitelabel   │ ✓ ON  │ Custom branding and white-label    │
└──────────────┴──────────┴────────────────────────────────────┘

Step 3: Attempting to use features...

─── Export Feature ───
  ✓ Successfully exported data to CSV format! (1,234 records)

─── Analytics Feature ───
  ✓ Generated Monthly Sales report with 15 charts and 42 insights!

─── API Access Feature ───
  ✓ API Key: sk_live_abc123xyz789 (use this in your integrations)

─── Premium Support Feature ───
  ✓ Priority support ticket created! Our team will respond within 1 hour.

Step 4: Releasing license...
  ✓ License released

=== Example Complete ===
```

## Key Code Patterns

### Checking a Feature

```rust
/// Helper to check if a license has a specific feature
fn license_has_feature(license: &License, feature: &str) -> bool {
    license
        .cached
        .as_ref()
        .map(|c| c.has_feature(feature))
        .unwrap_or(false)
}
```

### Feature-Gated Function

```rust
fn export_data(license: &License, format: &str) -> Result<String, String> {
    // Check feature FIRST, before doing any work
    if !license_has_feature(license, "export") {
        return Err(format!(
            "Export feature not available. Upgrade to access {} export.",
            format
        ));
    }

    // Feature is enabled - do the actual work
    Ok(format!("Successfully exported to {}", format))
}
```

### Feature-Gated UI Element (Conceptual)

```rust
fn render_export_button(license: &License) {
    if license_has_feature(license, "export") {
        // Show enabled button
        render_button("Export Data", on_export_click);
    } else {
        // Show disabled button with upgrade prompt
        render_disabled_button("Export Data (Upgrade Required)", on_upgrade_click);
    }
}
```

### Bulk Feature Check

```rust
fn get_available_features(license: &License) -> Vec<&'static str> {
    let all_features = ["basic", "export", "analytics", "premium", "api"];

    all_features
        .iter()
        .filter(|f| license_has_feature(license, f))
        .copied()
        .collect()
}
```

## Best Practices

1. **Check features early**: Validate feature access before starting expensive operations
2. **Provide clear upgrade paths**: When blocking a feature, tell users how to get access
3. **Cache validation results**: Don't call the server for every feature check
4. **Graceful degradation**: Some features can be partially available instead of completely blocked
5. **Log feature usage**: Track which features are used for product analytics

## Common Pricing Tiers

| Tier       | Features                                    |
|------------|---------------------------------------------|
| Free/Trial | `basic` only                                |
| Basic      | `basic`, `export`                           |
| Pro        | `basic`, `export`, `analytics`              |
| Enterprise | `basic`, `export`, `analytics`, `premium`, `api`, `whitelabel` |

## Next Steps

- See the [basic-client example](../basic-client/) for simpler license validation
- See the [air-gapped example](../air-gapped/) for offline validation
- Read the [Client Integration Guide](../../guide/client-integration.md) for complete documentation
