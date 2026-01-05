# Troubleshooting Guide

This guide covers common issues you might encounter when using Talos and how to resolve them.

## Table of Contents

- [Client Errors](#client-errors)
- [Server Errors](#server-errors)
- [Database Issues](#database-issues)
- [Authentication Problems](#authentication-problems)
- [Network Issues](#network-issues)
- [FAQ](#faq)

---

## Client Errors

### "License not found" (LICENSE_NOT_FOUND)

**Symptoms:**
- `ClientErrorCode::LicenseNotFound` error
- HTTP 404 response from server

**Causes:**
1. Typo in the license key
2. License was never created
3. Pointing to wrong server

**Solutions:**

```rust
// Double-check your license key
let license = License::new(
    "LIC-A1B2-C3D4-E5F6-G7H8".to_string(),  // Check this carefully
    "https://license.example.com".to_string(),
);
```

Verify the license exists via Admin API:
```bash
curl "https://license.example.com/api/v1/licenses?org_id=your-org" \
  -H "Authorization: Bearer <admin-token>"
```

---

### "Hardware mismatch" (HARDWARE_MISMATCH)

**Symptoms:**
- Validation fails with `HardwareMismatch` error
- License was working, then suddenly stopped

**Causes:**
1. License bound to a different machine
2. Hardware changed (new motherboard, CPU, or VM migration)
3. Running in a container with different hardware ID

**Solutions:**

**Option 1: Release from the other machine**

On the original machine:
```rust
license.release().await?;
```

**Option 2: Admin force-release**
```bash
curl -X POST "https://license.example.com/api/v1/licenses/{license_id}/release" \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{"reason": "User moved to new machine"}'
```

**Option 3: For containers/VMs**

If hardware ID changes frequently (cloud VMs, containers), consider:
- Using a consistent hardware identifier
- Implementing a more lenient binding policy

---

### "License already bound" (ALREADY_BOUND)

**Symptoms:**
- `bind()` fails with `AlreadyBound` error
- Error message includes the device name

**Causes:**
- License is bound to another machine
- Previous installation wasn't properly released

**Solutions:**

Check what device has the license:
```bash
curl "https://license.example.com/api/v1/licenses/{license_id}" \
  -H "Authorization: Bearer <admin-token>"
```

Response shows:
```json
{
  "is_bound": true,
  "device_name": "John's Old Laptop",
  "bound_at": "2024-01-15T10:00:00Z"
}
```

Then release it via admin API (see above).

---

### "License expired" (LICENSE_EXPIRED)

**Symptoms:**
- Validation fails with `LicenseExpired` error
- Was working before a certain date

**Solutions:**

**Option 1: Extend the license (Admin)**
```bash
curl -X POST "https://license.example.com/api/v1/licenses/{license_id}/extend" \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{"new_expires_at": "2026-12-31T23:59:59Z"}'
```

**Option 2: Create a new license**

For subscription models, create a new license when renewed.

---

### "Not bound" (NOT_BOUND)

**Symptoms:**
- `validate()` or `heartbeat()` fails with `NotBound` error

**Cause:**
- Calling `validate()` before `bind()`
- License was released

**Solution:**

Always bind before validating:
```rust
// Correct order
license.bind(Some("My Device"), None).await?;
license.validate().await?;
```

---

### "Feature not included" (FEATURE_NOT_INCLUDED)

**Symptoms:**
- `validate_feature()` returns error
- Feature check fails

**Cause:**
- Feature not in the license's tier
- License doesn't have that feature explicitly

**Solutions:**

Check what features the license has:
```rust
let result = license.validate().await?;
println!("Features: {:?}", result.features);

// Check locally
if result.has_feature("premium_export") {
    // Enable feature
}
```

Upgrade the license tier via Admin API:
```bash
curl -X PATCH "https://license.example.com/api/v1/licenses/{license_id}" \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{"tier": "enterprise"}'
```

---

### Offline validation fails

**Symptoms:**
- `validate_offline()` fails even though you validated recently
- "Cache not found" or similar error

**Causes:**
1. Never performed online validation (cache doesn't exist)
2. Cache file was deleted
3. Hardware ID changed (cache is hardware-bound)
4. Grace period expired

**Solutions:**

**Ensure online validation first:**
```rust
// This creates/updates the cache
license.validate().await?;

// Now offline works
license.validate_offline().await?;
```

**Check grace period:**
```rust
match license.validate_offline().await {
    Ok(result) => {
        if let Some(warning) = result.warning {
            // Grace period might be expiring
            println!("Warning: {}", warning);
        }
    }
    Err(e) => {
        // Need to go online
        println!("Offline validation failed: {}", e);
    }
}
```

---

## Server Errors

### Server won't start

**Symptoms:**
- `cargo run` fails
- "Address already in use" error
- Database connection errors

**Solutions:**

**Port in use:**
```bash
# Find what's using port 8080
lsof -i :8080
# or on Windows
netstat -ano | findstr :8080

# Use a different port
TALOS_SERVER_PORT=8081 cargo run
```

**Database connection:**
```bash
# Check database URL
echo $DATABASE_URL

# For SQLite, ensure directory exists
mkdir -p /var/lib/talos

# For PostgreSQL, test connection
psql $DATABASE_URL -c "SELECT 1"
```

---

### "Database error" (DATABASE_ERROR)

**Symptoms:**
- HTTP 500 responses
- "Database error" in logs

**Causes:**
1. Database connection lost
2. Migrations not run
3. Disk full (SQLite)
4. Connection pool exhausted

**Solutions:**

**Run migrations:**
```bash
export DATABASE_URL="sqlite://talos.db"
sqlx migrate run
```

**Check disk space:**
```bash
df -h
```

**Check PostgreSQL connections:**
```sql
SELECT count(*) FROM pg_stat_activity WHERE datname = 'talos';
```

---

### Rate limiting (HTTP 429)

**Symptoms:**
- "Too many requests" error
- HTTP 429 response

**Cause:**
- Client making too many requests
- Rate limit misconfigured

**Solutions:**

**Adjust rate limits:**
```toml
# config.toml
[rate_limit]
validate_per_minute = 200  # Increase if needed
heartbeat_per_minute = 120
bind_per_minute = 20
```

**Implement backoff in client:**
```rust
use tokio::time::{sleep, Duration};

async fn validate_with_backoff(license: &mut License) {
    let mut delay = Duration::from_secs(1);

    loop {
        match license.validate().await {
            Ok(result) => return,
            Err(e) if e.is_rate_limited() => {
                sleep(delay).await;
                delay *= 2;  // Exponential backoff
                if delay > Duration::from_secs(60) {
                    delay = Duration::from_secs(60);
                }
            }
            Err(e) => panic!("Validation failed: {}", e),
        }
    }
}
```

---

## Database Issues

### SQLite "database is locked"

**Symptoms:**
- "database is locked" errors
- Timeouts on writes

**Cause:**
- Multiple processes accessing the same SQLite file
- Long-running transactions

**Solutions:**

**Use PostgreSQL for production** - SQLite isn't designed for high concurrency.

**If you must use SQLite:**
```toml
# Increase timeout
[database]
sqlite_url = "sqlite://talos.db?mode=rwc&busy_timeout=30000"
```

---

### PostgreSQL connection refused

**Symptoms:**
- "Connection refused" error
- Can't reach database

**Solutions:**

**Check PostgreSQL is running:**
```bash
systemctl status postgresql
# or
docker ps | grep postgres
```

**Check connection settings:**
```bash
# Test connection
psql -h localhost -U talos -d talos

# Check pg_hba.conf allows connections
sudo cat /etc/postgresql/15/main/pg_hba.conf
```

**Check firewall:**
```bash
# Allow PostgreSQL port
sudo ufw allow 5432/tcp
```

---

## Authentication Problems

### "Invalid token" (INVALID_TOKEN)

**Symptoms:**
- HTTP 401 response
- "Invalid token" error

**Causes:**
1. Token malformed
2. Wrong JWT secret
3. Token from different environment

**Solutions:**

**Verify token format:**
```bash
# Token should be three base64 parts separated by dots
echo "eyJhbGciOiJIUzI1NiIs..." | cut -d. -f2 | base64 -d
```

**Check JWT secret matches:**
```bash
# Server side
echo $TALOS_JWT_SECRET

# Ensure same secret used to create and verify tokens
```

---

### "Token expired" (TOKEN_EXPIRED)

**Symptoms:**
- HTTP 401 response
- "Token expired" error
- Was working, then stopped

**Solutions:**

**Create a new token:**
```bash
curl -X POST "https://license.example.com/api/v1/tokens" \
  -H "Authorization: Bearer <valid-token>" \
  -H "Content-Type: application/json" \
  -d '{"name": "new-token", "scopes": ["licenses:*"]}'
```

**Increase token lifetime:**
```toml
# config.toml
[auth]
token_expiration_secs = 604800  # 7 days
```

---

### "Insufficient scope" (INSUFFICIENT_SCOPE)

**Symptoms:**
- HTTP 403 response
- Can read but not write (or vice versa)

**Cause:**
- Token doesn't have required permissions

**Solution:**

Create a token with correct scopes:
```bash
curl -X POST "https://license.example.com/api/v1/tokens" \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "full-access",
    "scopes": ["licenses:*", "tokens:*"]
  }'
```

---

## Network Issues

### Connection timeouts

**Symptoms:**
- Requests hang then fail
- "Connection timed out" errors

**Solutions:**

**Check server is reachable:**
```bash
curl -v https://license.example.com/health
```

**Check firewall:**
```bash
# Allow Talos port
sudo ufw allow 8080/tcp
```

**Check DNS:**
```bash
nslookup license.example.com
```

---

### SSL/TLS errors

**Symptoms:**
- "Certificate verify failed"
- SSL handshake errors

**Solutions:**

**Check certificate:**
```bash
openssl s_client -connect license.example.com:443 -servername license.example.com
```

**For self-signed certs (development only):**
```rust
// NOT for production!
let client = reqwest::Client::builder()
    .danger_accept_invalid_certs(true)
    .build()?;
```

---

## FAQ

### Can I use the same license on multiple machines?

No, a license can only be bound to one machine at a time. This is by design to prevent license sharing. If you need multiple machines, create multiple licenses or implement a floating license system.

### How do I transfer a license to a new machine?

1. Release the license on the old machine: `license.release().await?`
2. Or use admin API to force-release
3. Bind on the new machine: `license.bind(...).await?`

### What happens if my server goes down?

If you've enabled offline validation:
1. Clients continue working using cached validation
2. Cache is valid for the configured grace period
3. Once grace period expires, validation fails

Recommendation: Configure a grace period appropriate for your SLA.

### How often should I send heartbeats?

Every 5-15 minutes is typical. Heartbeats:
- Update the grace period for offline validation
- Let you detect inactive licenses
- Don't stress the server significantly

### Can I run Talos without authentication?

Yes, for development. **Never do this in production.** Anyone could create, modify, or delete licenses.

### How do I debug validation issues?

Enable debug logging:
```bash
RUST_LOG=debug cargo run
```

On the client:
```rust
// Check what the server returns
match license.validate().await {
    Ok(result) => println!("{:?}", result),
    Err(e) => {
        println!("Error: {}", e);
        if let Some(api_error) = e.as_api_error() {
            println!("Code: {:?}", api_error.code);
            println!("Message: {}", api_error.message);
        }
    }
}
```

### How do I handle license expiration in my app?

```rust
match license.validate().await {
    Ok(result) => {
        if let Some(expires_at) = &result.expires_at {
            let expiry = chrono::DateTime::parse_from_rfc3339(expires_at)?;
            let days_left = (expiry - chrono::Utc::now()).num_days();

            if days_left < 30 {
                show_renewal_reminder(days_left);
            }
        }
    }
    Err(e) if matches!(e.as_api_error().map(|e| &e.code), Some(ClientErrorCode::LicenseExpired)) => {
        show_expired_dialog();
        disable_app();
    }
    Err(e) => {
        // Handle other errors
    }
}
```

---

## Still Need Help?

- **GitHub Issues**: [github.com/dmriding/talos/issues](https://github.com/dmriding/talos/issues)
- **Discussions**: [github.com/dmriding/talos/discussions](https://github.com/dmriding/talos/discussions)
- **Check the logs**: Most issues can be diagnosed from server logs with `RUST_LOG=debug`
