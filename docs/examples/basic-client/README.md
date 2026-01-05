# Basic Talos Client Example

This example demonstrates the core Talos client functionality with a **realistic license key entry flow** - no recompilation required to change license keys.

## How License Keys Work

**License keys are NOT compiled into your application.** They are entered at runtime:

1. **User purchases a license** → Receives a key like `LIC-A1B2-C3D4-E5F6-G7H8`
2. **User runs your app** → App prompts for the license key (first run only)
3. **App saves the key** → Key is stored locally for future runs
4. **App calls bind()** → Associates the key with the user's hardware
5. **Done** → License is active, no recompilation needed

## License Key Sources

This example supports multiple ways to provide the license key (in order of priority):

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
Invoke-RestMethod -Uri http://127.0.0.1:8080/api/v1/licenses -Method POST -ContentType "application/json" -Body '{"org_id": "example-org", "features": ["basic", "export"], "expires_at": "2030-12-31T23:59:59Z"}'
```

**curl (bash/WSL):**
```bash
curl -X POST http://127.0.0.1:8080/api/v1/licenses \
  -H "Content-Type: application/json" \
  -d '{"org_id": "example-org", "features": ["basic", "export"], "expires_at": "2030-12-31T23:59:59Z"}'
```

Note the `license_key` from the response (e.g., `LIC-A1B2-C3D4-E5F6-G7H8`).

### 3. Run the Example

**Option A: Let it prompt you**
```bash
cargo run
# Enter your license key when prompted
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

**Option C: Config file** (for subsequent runs)

*PowerShell (Windows):*
```powershell
"LIC-A1B2-C3D4-E5F6-G7H8" | Out-File -Encoding ascii license.key
cargo run
```

*bash (Mac/Linux):*
```bash
echo "LIC-A1B2-C3D4-E5F6-G7H8" > license.key
cargo run
```

## Expected Output

```
=== Talos Basic Client Example ===

Using license key from LICENSE_KEY environment variable
Server URL: http://127.0.0.1:8080
License Key: LIC-A1B2-C3D4...

Step 1: Binding license to this machine...
  ✓ License bound successfully!
    License ID: 550e8400-e29b-41d4-a716-446655440000
    Features: ["basic", "export"]
    Expires: 2030-12-31T23:59:59Z

Step 2: Validating license...
  ✓ License is valid!
    Features: ["basic", "export"]

Step 3: Checking features...
    basic - ✓ Enabled
    export - ✓ Enabled
    premium - ✗ Disabled
    analytics - ✗ Disabled

Step 4: Sending heartbeat...
  ✓ Heartbeat sent!
    Server time: 2024-01-15T10:30:00Z

Step 5: Testing offline validation...
  ✗ Offline validation failed: Grace period expired
    (This is expected - see note below)

Step 6: Releasing license...
  ✓ License released successfully!
    The license can now be used on another machine.

=== Example Complete ===
```

## Note on Offline Validation

Step 5 (offline validation) will fail for **active licenses** because offline validation only works when:

- The license is **suspended** with an explicit grace period
- The admin has set `grace_period_ends_at` via the Admin API

This is by design - the server controls when offline access is permitted. For applications that need offline validation, see the [air-gapped example](../air-gapped/) which demonstrates the full workflow including license suspension with a grace period.

## Changing License Keys

To use a different license key:

**PowerShell (Windows):**
```powershell
# Delete the saved key
Remove-Item license.key

# Run again - it will prompt for a new key
cargo run
```

**bash (Mac/Linux):**
```bash
# Delete the saved key
rm license.key

# Run again - it will prompt for a new key
cargo run
```

Or use the environment variable to override:

**PowerShell (Windows):**
```powershell
$env:LICENSE_KEY="LIC-NEW-KEY-HERE"; cargo run
```

**bash (Mac/Linux):**
```bash
LICENSE_KEY="LIC-NEW-KEY-HERE" cargo run
```

## Key Code Patterns

### Getting the license key at runtime

```rust
fn get_license_key() -> io::Result<String> {
    // 1. Check environment variable
    if let Ok(key) = std::env::var("LICENSE_KEY") {
        return Ok(key);
    }

    // 2. Check saved file
    if let Ok(key) = std::fs::read_to_string("license.key") {
        return Ok(key.trim().to_string());
    }

    // 3. Prompt user
    print!("Enter license key: ");
    let mut key = String::new();
    io::stdin().read_line(&mut key)?;

    // Save for next time
    std::fs::write("license.key", key.trim())?;

    Ok(key.trim().to_string())
}
```

### Creating the License instance

```rust
// License key is a runtime String, NOT a compile-time constant
let license_key = get_license_key()?;
let server_url = std::env::var("SERVER_URL").unwrap_or("http://localhost:8080".into());

let mut license = License::new(license_key, server_url);
```

## Production Recommendations

1. **Store keys securely**: Use OS keychain, encrypted config, or secure storage
2. **Use environment variables in containers**: `LICENSE_KEY` env var works great for Docker
3. **Implement a license dialog**: For GUI apps, show a proper dialog on first run
4. **Handle key changes**: Let users update their key without reinstalling

## Next Steps

- See the [air-gapped example](../air-gapped/) for offline-first applications
- Read the [Client Integration Guide](../../guide/client-integration.md) for complete documentation
