# Getting Started with Talos

This guide will get you up and running with Talos in about 5 minutes. By the end, you'll have a working license server and a client that can bind, validate, and release licenses.

## Prerequisites

- **Rust** (stable, 1.70+)
- **SQLx CLI** (for database migrations)

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install SQLx CLI
cargo install sqlx-cli
```

## Quick Start

### 1. Add Talos to Your Project

Add Talos to your `Cargo.toml`:

```toml
[dependencies]
talos = { git = "https://github.com/dmriding/talos" }
tokio = { version = "1", features = ["full"] }
```

**Feature Flags:** Talos uses feature flags to keep your binary lean. The defaults (`server`, `sqlite`) work for most use cases.

| Feature | Default | Description |
|---------|---------|-------------|
| `server` | Yes | Server components (handlers, database) |
| `sqlite` | Yes | SQLite database backend |
| `postgres` | No | PostgreSQL database backend |
| `admin-api` | No | Admin CRUD API for license management |
| `jwt-auth` | No | JWT authentication for protected endpoints |
| `rate-limiting` | No | Rate limiting middleware |
| `background-jobs` | No | Scheduled background jobs |
| `openapi` | No | OpenAPI 3.0 spec and Swagger UI |

**Common configurations:**

```toml
# Client-only (no server components)
talos = { git = "https://github.com/dmriding/talos", default-features = false }

# Server with admin API
talos = { git = "https://github.com/dmriding/talos", features = ["admin-api"] }

# Full-featured server
talos = { git = "https://github.com/dmriding/talos", features = ["admin-api", "jwt-auth", "rate-limiting", "openapi"] }
```

### 2. Set Up the Database

Create a configuration file `config.toml`:

```toml
[server]
host = "127.0.0.1"
port = 8080

[database]
db_type = "sqlite"
sqlite_url = "sqlite://talos.db"
```

Run the database migrations:

```bash
# Set the database URL
export DATABASE_URL="sqlite://talos.db"

# Run migrations
sqlx migrate run
```

> **Alternative:** For quick setup without SQLx, you can use the standalone script:
> ```bash
> sqlite3 talos.db < scripts/sql/init_sqlite.sql
> ```

### 3. Start the Server

Create a simple server binary (`src/bin/server.rs`):

```rust
use talos::server::{build_router, AppState};
use talos::server::database::Database;
use talos::config::get_config;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = get_config();

    // Connect to database
    let db = Database::connect(&config.database).await?;
    let db = Arc::new(db);

    // Build the router
    let app = build_router(AppState {
        db,
        #[cfg(feature = "jwt-auth")]
        auth: talos::server::auth::AuthState::disabled(),
    });

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    println!("Talos server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

Run the server:

```bash
cargo run --bin server
```

### 4. Create Your First License (Admin API)

If you enabled the `admin-api` feature, you can create licenses via HTTP:

```bash
curl -X POST http://127.0.0.1:8080/api/v1/licenses \
  -H "Content-Type: application/json" \
  -d '{
    "org_id": "my-company",
    "features": ["basic", "export"],
    "expires_at": "2025-12-31T23:59:59Z"
  }'
```

Response:

```json
{
  "license_id": "550e8400-e29b-41d4-a716-446655440000",
  "license_key": "LIC-A1B2-C3D4-E5F6-G7H8",
  "org_id": "my-company",
  "features": ["basic", "export"],
  "status": "active",
  "expires_at": "2025-12-31T23:59:59Z"
}
```

Save the `license_key` - your clients will use this to bind.

### 5. Use the Client Library

Now integrate Talos into your application:

```rust
use talos::client::License;
use talos::errors::LicenseResult;

#[tokio::main]
async fn main() -> LicenseResult<()> {
    // Create a license client with your license key
    let mut license = License::new(
        "LIC-A1B2-C3D4-E5F6-G7H8".to_string(),
        "http://127.0.0.1:8080".to_string(),
    );

    // Bind the license to this machine
    let bind_result = license.bind(
        Some("My Workstation"),  // Device name (optional)
        Some("Windows 11 PC"),   // Device info (optional)
    ).await?;

    println!("License bound successfully!");
    println!("Features: {:?}", bind_result.features);
    println!("Expires: {:?}", bind_result.expires_at);

    // Validate the license (do this periodically)
    let validation = license.validate().await?;

    if validation.has_feature("export") {
        println!("Export feature is enabled!");
        // Enable export functionality
    }

    // Send heartbeats to keep the license active
    let heartbeat = license.heartbeat().await?;
    println!("Server time: {}", heartbeat.server_time);

    // When your app closes, release the license
    license.release().await?;
    println!("License released");

    Ok(())
}
```

## What's Next?

You now have a working Talos setup! Here's where to go from here:

- **[Client Integration Guide](client-integration.md)** - Deep dive into all client methods, error handling, and offline validation
- **[Server Deployment Guide](server-deployment.md)** - Production deployment with PostgreSQL, Docker, and TLS
- **[Admin API Guide](admin-api.md)** - Full admin API documentation for license management
- **[Advanced Topics](advanced.md)** - Custom hardware fingerprinting, background jobs, rate limiting

## Troubleshooting

### "License not found" error

Make sure:
1. The license key is correct (check for typos)
2. The server is running and reachable
3. The license was created via the admin API

### "Hardware mismatch" error

The license is bound to a different machine. Use the admin API to release it:

```bash
curl -X POST http://127.0.0.1:8080/api/v1/licenses/{license_id}/release \
  -H "Content-Type: application/json" \
  -d '{"reason": "User requested transfer"}'
```

### Database connection issues

Check your `config.toml` and ensure:
- The database file path is correct (for SQLite)
- The database URL is correct (for PostgreSQL)
- Migrations have been run (`sqlx migrate run`)

### Need more help?

- Check the [Troubleshooting Guide](troubleshooting.md)
- Open an issue on [GitHub](https://github.com/dmriding/talos/issues)
- Join the discussion on [GitHub Discussions](https://github.com/dmriding/talos/discussions)
