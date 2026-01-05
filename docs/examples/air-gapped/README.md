# Air-Gapped/Offline Validation Example

This example demonstrates how to use Talos in environments where network connectivity is intermittent or unavailable (air-gapped systems, industrial controllers, field deployments).

## Important: When Offline Validation Works

Offline validation **only works for licenses with an explicit grace period**. This happens when:

- A license is **suspended** with a grace period (e.g., pending payment, scheduled maintenance)
- The admin explicitly sets `grace_period_ends_at` via the Admin API

**Active licenses do NOT have offline validation by default.** This is by design - the server controls when offline access is permitted.

## How It Works

1. **Initial Activation** (requires network): Bind and validate once to populate the encrypted cache
2. **License Suspension**: Admin suspends the license with a grace period via the Admin API
3. **Offline Validation**: Use cached license data when network is unavailable during grace period
4. **Periodic Refresh**: During maintenance windows, reconnect to extend the grace period

## License Keys Are Runtime Input

**License keys are NOT compiled into your application.** They are entered at runtime:

1. **Environment variable**:
   - PowerShell: `$env:LICENSE_KEY="LIC-XXXX"; cargo run`
   - bash: `LICENSE_KEY=LIC-XXXX cargo run`
2. **Saved file**: Reads from `license.key` if it exists
3. **User prompt**: Asks the user to enter their key (first run)

## Running the Example

### 1. Start the Talos Server

```bash
# In the main Talos directory
cargo run --bin talos_server --features admin-api
```

### 2. Create a License

**PowerShell:**
```powershell
Invoke-RestMethod -Uri http://127.0.0.1:8080/api/v1/licenses -Method POST -ContentType "application/json" -Body '{"org_id": "industrial-corp", "features": ["basic", "export", "advanced"], "expires_at": "2030-12-31T23:59:59Z"}'
```

**curl (bash/WSL):**
```bash
curl -X POST http://127.0.0.1:8080/api/v1/licenses \
  -H "Content-Type: application/json" \
  -d '{"org_id": "industrial-corp", "features": ["basic", "export", "advanced"], "expires_at": "2030-12-31T23:59:59Z"}'
```

Note the `license_key` AND `license_id` from the response. You'll need the `license_id` (UUID) for step 3.

### 3. Suspend the License with a Grace Period

For offline validation to work, you must suspend the license with a grace period. Use the Admin API:

```bash
# PowerShell
Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/licenses/YOUR_LICENSE_ID/revoke" -Method POST -ContentType "application/json" -Body '{"grace_period_days": 30}'

# curl (bash/WSL)
curl -X POST "http://127.0.0.1:8080/api/v1/licenses/YOUR_LICENSE_ID/revoke" \
  -H "Content-Type: application/json" \
  -d '{"grace_period_days": 30}'
```

Replace `YOUR_LICENSE_ID` with the `license_id` from step 2 (NOT the license_key).

This sets the license to "suspended" status with a 30-day grace period during which offline validation will work.

### 4. Run the Example (Online Mode - Caches License Data)

**Option A: Let it prompt you**
```bash
cargo run
```

**Option B: Environment variable**

*PowerShell (Windows):*
```powershell
$env:LICENSE_KEY="LIC-A1B2-C3D4-E5F6-G7H8"; cargo run
```

*bash (Mac/Linux):*
```bash
LICENSE_KEY="LIC-A1B2-C3D4-E5F6-G7H8" cargo run
```

### 5. Test True Offline Mode

Now stop the server and run with the `--offline` flag:

```bash
# Stop the server (Ctrl+C in the server terminal)

# Run in offline mode - no server connection required!
cargo run -- --offline
```

This demonstrates that the license validation works entirely from the encrypted local cache, without any network connection.

### 6. (Optional) Reinstate the License

After testing, you can reinstate the license to active status:

```bash
# PowerShell
Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/licenses/YOUR_LICENSE_ID/reinstate" -Method POST -ContentType "application/json" -Body '{}'

# curl
curl -X POST "http://127.0.0.1:8080/api/v1/licenses/YOUR_LICENSE_ID/reinstate" \
  -H "Content-Type: application/json" \
  -d '{}'
```

## Expected Output

### Online Mode (Step 4)

```
=== Talos Air-Gapped Example ===

This will connect to the server to cache license data.
After running, use '--offline' flag to test offline mode:

  cargo run -- --offline

Using license key from LICENSE_KEY environment variable
Server URL: http://127.0.0.1:8080
License Key: LIC-A1B2-C3D4...

Phase 1: Initial Online Activation
-----------------------------------
Binding license...
  ✓ License bound!
    Features: ["basic", "export", "advanced"]
Validating and caching license data...
  ✓ License validated and cached!
    Features: ["basic", "export", "advanced"]
    Grace period until: 2026-02-04T21:30:14+00:00
Extending grace period with heartbeat...
  ✓ Grace period extended!

Phase 2: Offline Operation (Simulated)
--------------------------------------
Testing offline validation with cached data...

Method 1: Direct offline validation
  ✓ Offline validation successful!
    Features: ["basic", "export", "advanced"]
    ⚠ Warning: Offline mode - license must be validated online before 2026-02-04T21:30:14+00:00

Feature Gating (from cache):
  ✓ basic
  ✓ export
  ✓ advanced
  ✗ premium

Grace Period Status:
  Expires: 2026-02-04T21:30:14+00:00
  Status: Valid for offline use

=== Best Practices for Air-Gapped Systems ===
...

=== Example Complete ===

The license cache has been saved to disk.
Now stop the server and run:

  cargo run -- --offline

This will demonstrate true offline validation!
```

### Offline Mode (Step 5)

```
=== Talos Air-Gapped Example (OFFLINE MODE) ===

Running in offline mode - server connection not required.

Loading cached license data from disk...
  ✓ Cached license data loaded!
    License Key: LIC-A1B2-C3D4...

=== Offline Validation Demo ===
(Server is not being contacted)

Method 1: Direct offline validation
  ✓ Offline validation successful!
    Features: ["basic", "export", "advanced"]
    ⚠ Warning: Offline mode - license must be validated online before 2026-02-04T21:30:14+00:00

Feature Gating (from cache):
  ✓ basic
  ✓ export
  ✓ advanced
  ✗ premium

Grace Period Status:
  Expires: 2026-02-04T21:30:14+00:00
  Status: Valid for offline use

=== Best Practices for Air-Gapped Systems ===
...

=== Offline Demo Complete ===

The license was validated entirely from the encrypted local cache.
No network connection was required!
```

## Key Code Patterns

### Validation with Automatic Fallback

```rust
// Tries online first, falls back to cached offline validation
match license.validate_with_fallback().await {
    Ok(result) => {
        println!("Features: {:?}", result.features);

        if let Some(warning) = &result.warning {
            // Warning indicates we're using offline validation
            // or grace period is running low
            handle_warning(warning);
        }
    }
    Err(e) => {
        // Both online and offline failed
        // Grace period may have expired
        enter_limited_mode();
    }
}
```

### Direct Offline Validation

```rust
// Use when you know network is unavailable
// Note: validate_offline() is synchronous (no await)
match license.validate_offline() {
    Ok(result) => {
        // Check grace period status
        if let Some(grace_end) = &result.grace_period_ends_at {
            let remaining = calculate_days_remaining(grace_end);
            if remaining < 7 {
                notify_maintenance_needed(remaining);
            }
        }
    }
    Err(e) => {
        // Cache missing or grace period expired
        // Network connectivity required
    }
}
```

### Extending Grace Period

```rust
// Call during maintenance windows when network is available
match license.heartbeat().await {
    Ok(result) => {
        if let Some(grace_end) = &result.grace_period_ends_at {
            println!("Grace period extended until: {}", grace_end);
        }
    }
    Err(_) => {
        // Network unavailable, try again later
    }
}
```

## Use Cases

- **Industrial Systems**: PLCs, SCADA systems, factory controllers
- **Field Deployments**: Remote sensors, edge devices, kiosks
- **High-Security Environments**: Air-gapped networks, classified systems
- **Desktop Applications**: Professional software that should work offline

## Cache Security

The offline cache is protected by:

1. **AES-256-GCM Encryption**: Industry-standard authenticated encryption
2. **Hardware-Bound Key**: Derived from machine-specific identifiers
3. **Tamper Detection**: GCM authentication tag prevents modification
4. **Non-Transferable**: Cache cannot be copied to another machine

## Production Recommendations

1. **Plan Maintenance Windows**: Schedule network access before grace period expires
2. **Monitor Warnings**: Log and alert on grace period warnings
3. **Graceful Degradation**: Define what happens when license expires offline
4. **Initial Activation**: Ensure first-run has network access for binding

## Next Steps

- See the [basic-client example](../basic-client/) for simpler online-only usage
- Read the [Client Integration Guide](../../guide/client-integration.md) for complete documentation
