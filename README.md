# Talos — Secure Rust Licensing System

[![Crates.io](https://img.shields.io/crates/v/netviper-talos.svg)](https://crates.io/crates/netviper-talos)
[![Documentation](https://docs.rs/netviper-talos/badge.svg)](https://docs.rs/netviper-talos)
[![Downloads](https://img.shields.io/crates/d/netviper-talos.svg)](https://crates.io/crates/netviper-talos)
[![Build Status](https://github.com/dmriding/talos/actions/workflows/ci.yml/badge.svg)](https://github.com/dmriding/talos/actions)
[![codecov](https://codecov.io/gh/dmriding/talos/graph/badge.svg)](https://codecov.io/gh/dmriding/talos)
[![License](https://img.shields.io/badge/license-MIT-blue)](https://github.com/dmriding/talos/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)

**Talos** is a Rust-based secure licensing framework providing:

- Hardware-bound licensing
- Encrypted license storage
- License activation/validation
- Heartbeat-based liveness checks
- A lightweight Axum server backend
- A robust async client library

Your software gets a reliable, secure "gatekeeper," inspired by **Talos**, the mythological bronze guardian who protected Crete.

---

## Overview

Talos offers:

- A **client library** for embedding license logic inside your application
- A **server component** for verifying, activating, and tracking licenses
- Full async, cross-platform compatibility (Windows, macOS, Linux)
- Strong cryptography (AES-256-GCM + SHA-256 hardware fingerprinting)

Talos is built to be **easy to integrate yet extremely hard to bypass**.

---

## Key Features

- **Hardware Binding** — Licenses tied to CPU + motherboard fingerprint
- **AES-256-GCM Encrypted Storage** — License files encrypted locally
- **Networked License Control** — Activate/validate/deactivate remotely
- **Heartbeat System** — Keeps licenses alive and trackable
- **SQLite & Postgres Support** — Choose your storage backend
- **Self-Hostable** — Easy to deploy, zero external dependencies
- **Fully Async** — Powered by `tokio`, `axum`, `reqwest`, `sqlx`
- **Strong Test Coverage** — Unit tests + integration tests

---

## How It Works

1. Talos generates a **hardware fingerprint** using CPU + motherboard identifiers (hashed via SHA-256).
2. License data is encrypted locally using **AES-256-GCM**.
3. Client communicates with the server using HTTPS via `reqwest`.
4. Server stores licenses and heartbeat timestamps via SQLx.
5. A small REST API allows:
   - Activation
   - Validation
   - Deactivation
   - Heartbeat updates

---

## Project Structure

```plaintext
talos/
├── src/
│   ├── client/
│   │   ├── license.rs            # License struct + client operations
│   │   ├── encrypted_storage.rs  # AES-256-GCM encrypted local storage
│   │   ├── heartbeat.rs          # Heartbeat HTTP operations
│   │   ├── key_generation.rs     # Device key helpers
│   │   └── main.rs               # Example client binary
│   ├── server/
│   │   ├── database.rs           # SQLite/Postgres abstraction
│   │   ├── handlers.rs           # Axum handlers for /activate, /validate...
│   │   ├── admin.rs              # Admin API handlers (feature-gated)
│   │   ├── auth.rs               # JWT authentication (feature-gated)
│   │   ├── routes.rs             # Router builder
│   │   ├── server_sim.rs         # In-memory simulation for tests
│   │   └── main.rs               # Server binary
│   ├── config.rs                 # Config loader (config.toml + env vars)
│   ├── encryption.rs             # AES-256-GCM utilities
│   ├── errors.rs                 # Custom LicenseError type
│   ├── hardware.rs               # Cross-platform hardware fingerprinting
│   ├── license_key.rs            # License key generation/validation
│   ├── tiers.rs                  # Tier configuration system
│   └── lib.rs                    # Library entry point
├── tests/                        # Unit and integration tests
├── examples/                     # Usage examples
├── migrations/                   # Database migrations
├── docs/
│   ├── public/
│   │   └── ROADMAP.md            # Development roadmap
│   ├── api/                      # API reference documentation
│   │   ├── rest-api.md           # Complete REST API reference
│   │   └── openapi.json          # OpenAPI 3.1.0 specification
│   ├── examples/                 # Complete runnable examples
│   │   ├── basic-client/         # Minimal client integration
│   │   ├── air-gapped/           # Offline validation example
│   │   └── feature-gating/       # Feature gating example
│   └── guide/                    # User guides
│       ├── getting-started.md    # 5-minute quickstart
│       ├── client-integration.md # Client library usage
│       ├── server-deployment.md  # Production deployment
│       ├── admin-api.md          # Admin API guide
│       └── troubleshooting.md    # Debugging and FAQ
├── .claude/
│   └── README.md                 # AI assistant context (for contributors)
├── config.toml.example           # Example configuration
├── .env.example                  # Example environment variables
├── Cargo.toml
└── README.md
```

---

## Prerequisites

- Rust (stable)
- SQLite **or** PostgreSQL
- `sqlx-cli` (for running migrations)

Install Rust:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update
```

Install SQLx CLI:

```sh
cargo install sqlx-cli
```

---

## Installation

Add Talos to your project:

```toml
[dependencies]
talos = "0.1"
```

Then:

```sh
cargo build
```

---

## Feature Flags

Talos uses Cargo feature flags to let you include only what you need:

| Feature | Default | Description |
|---------|---------|-------------|
| `server` | Yes | Server components (handlers, database) |
| `sqlite` | Yes | SQLite database backend |
| `postgres` | No | PostgreSQL database backend |
| `jwt-auth` | No | JWT authentication middleware for protected endpoints |
| `admin-api` | No | Admin CRUD API for license management |
| `rate-limiting` | No | Rate limiting middleware for abuse prevention |
| `background-jobs` | No | Scheduled background jobs for license maintenance |
| `openapi` | No | OpenAPI 3.0 specification and Swagger UI |

### Examples

```toml
# Default: server + SQLite
talos = "0.1"

# Client-only (no server components)
talos = { version = "0.1", default-features = false }

# Server with PostgreSQL instead of SQLite
talos = { version = "0.1", default-features = false, features = ["server", "postgres"] }

# Server with both SQLite and PostgreSQL
talos = { version = "0.1", features = ["postgres"] }

# Full server with admin API and JWT auth
talos = { version = "0.1", features = ["admin-api", "jwt-auth"] }

# Server with background jobs enabled
talos = { version = "0.1", features = ["background-jobs"] }

# Full-featured server
talos = { version = "0.1", features = ["admin-api", "jwt-auth", "rate-limiting", "background-jobs"] }

# Server with OpenAPI documentation
talos = { version = "0.1", features = ["admin-api", "openapi"] }
```

---

## Quick Start

### 1. Configure

Copy the example configuration files:

```sh
cp config.toml.example config.toml
cp .env.example .env
```

Edit `config.toml` as needed:

```toml
server_url = "http://127.0.0.1:8080"
heartbeat_interval = 60
enable_logging = true

[database]
db_type = "sqlite"
sqlite_url = "sqlite://talos.db"
```

### 2. Run Migrations

```sh
sqlx migrate run
```

### 3. Start the Server

```sh
cargo run --bin talos_server
```

### 4. Use the Client

```rust
use talos::client::License;
use talos::errors::LicenseResult;

#[tokio::main]
async fn main() -> LicenseResult<()> {
    // Create a new license client
    let mut license = License::new(
        "LIC-XXXX-XXXX-XXXX".to_string(),  // Your license key
        "http://127.0.0.1:8080".to_string(),
    );

    // Bind to this hardware (replaces activate())
    let bind_result = license.bind(Some("My Workstation"), None).await?;
    println!("License bound. Features: {:?}", bind_result.features);

    // Validate the license
    let validation = license.validate().await?;
    if validation.has_feature("feature_a") {
        println!("Feature A is enabled!");
    }

    // Check a specific feature
    let feature_result = license.validate_feature("premium_export").await?;
    if feature_result.allowed {
        println!("Premium export is available");
    }

    // Send heartbeat
    let heartbeat = license.heartbeat().await?;
    println!("Server time: {}", heartbeat.server_time);

    // Release when done (replaces deactivate())
    license.release().await?;

    Ok(())
}
```

### Air-Gapped Systems

For systems that need to operate offline within a grace period:

```rust
use talos::client::License;

async fn check_license_with_fallback() -> bool {
    let mut license = License::load_from_disk().await.unwrap();

    // Try online validation first, fall back to cached offline validation
    match license.validate_with_fallback().await {
        Ok(result) => {
            if let Some(warning) = &result.warning {
                // System should go online soon
                eprintln!("Warning: {}", warning);
            }
            true
        }
        Err(e) => {
            eprintln!("License validation failed: {}", e);
            false
        }
    }
}
```

Or run the provided example:

```sh
cargo run --example manual_activate
```

---

## Server API Endpoints

### OpenAPI Documentation (requires `openapi` feature)

When running with the `openapi` feature enabled, interactive API documentation is available:

| Endpoint                | Description                        |
|-------------------------|------------------------------------|
| `/swagger-ui`           | Swagger UI for interactive API exploration |
| `/api-docs/openapi.json`| OpenAPI 3.0 specification (JSON)   |

Run the server with OpenAPI enabled:

```sh
cargo run --bin talos_server --features "openapi,admin-api"
```

Then navigate to `http://127.0.0.1:8080/swagger-ui` in your browser.

### System Endpoints (always available)

| Method | Endpoint      | Description                        |
|--------|---------------|------------------------------------|
| GET    | `/health`     | Health check with database status  |

The health endpoint returns:

```json
{
  "status": "healthy",
  "service": "talos",
  "version": "0.1.0",
  "database": {
    "connected": true,
    "db_type": "sqlite"
  }
}
```

### Legacy Client Endpoints (always available)

| Method | Endpoint      | Description              |
|--------|---------------|--------------------------|
| POST   | `/activate`   | Activate a license       |
| POST   | `/validate`   | Validate if license is active |
| POST   | `/deactivate` | Deactivate a license     |
| POST   | `/heartbeat`  | Send heartbeat ping      |

### Client API v1 Endpoints (always available)

| Method | Endpoint                        | Description                     |
|--------|---------------------------------|---------------------------------|
| POST   | `/api/v1/client/bind`           | Bind license to hardware        |
| POST   | `/api/v1/client/release`        | Release license from hardware   |
| POST   | `/api/v1/client/validate`       | Validate a license              |
| POST   | `/api/v1/client/validate-or-bind` | Validate or auto-bind         |
| POST   | `/api/v1/client/heartbeat`      | Send heartbeat                  |
| POST   | `/api/v1/client/validate-feature` | Validate feature access       |

### Admin Endpoints (requires `admin-api` feature)

| Method | Endpoint                              | Description                        |
|--------|---------------------------------------|------------------------------------|
| POST   | `/api/v1/licenses`                    | Create a new license               |
| POST   | `/api/v1/licenses/batch`              | Batch create licenses              |
| GET    | `/api/v1/licenses/{id}`               | Get license by ID                  |
| GET    | `/api/v1/licenses?org_id=X`           | List licenses by org               |
| PATCH  | `/api/v1/licenses/{id}`               | Update a license                   |
| POST   | `/api/v1/licenses/{id}/revoke`        | Revoke a license (with optional grace period) |
| POST   | `/api/v1/licenses/{id}/reinstate`     | Reinstate a suspended/revoked license |
| POST   | `/api/v1/licenses/{id}/extend`        | Extend license expiration          |
| PATCH  | `/api/v1/licenses/{id}/usage`         | Update usage/bandwidth metrics     |
| POST   | `/api/v1/licenses/{id}/release`       | Release hardware binding           |
| POST   | `/api/v1/licenses/{id}/blacklist`     | Permanently blacklist a license    |

### Token Endpoints (requires `admin-api` feature)

| Method | Endpoint                 | Description              |
|--------|--------------------------|--------------------------|
| POST   | `/api/v1/tokens`         | Create a new API token   |
| GET    | `/api/v1/tokens`         | List all API tokens      |
| GET    | `/api/v1/tokens/{id}`    | Get token details        |
| DELETE | `/api/v1/tokens/{id}`    | Revoke a token           |

All legacy client requests use:

```json
{
  "license_id": "LICENSE-12345",
  "client_id": "CLIENT-67890"
}
```

### Example

```sh
curl -X POST http://127.0.0.1:8080/validate \
  -H "Content-Type: application/json" \
  -d '{"license_id":"LICENSE-12345","client_id":"CLIENT-67890"}'
```

---

## Error Response Format

All API endpoints return errors in a standardized JSON format:

```json
{
  "error": {
    "code": "LICENSE_NOT_FOUND",
    "message": "The requested license does not exist",
    "details": null
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| **License State** |||
| `LICENSE_NOT_FOUND` | 404 | License key does not exist |
| `LICENSE_EXPIRED` | 403 | License has expired |
| `LICENSE_REVOKED` | 403 | License has been revoked |
| `LICENSE_SUSPENDED` | 403 | License is temporarily suspended |
| `LICENSE_BLACKLISTED` | 403 | License is permanently blacklisted |
| `LICENSE_INACTIVE` | 403 | License is not active |
| **Hardware Binding** |||
| `ALREADY_BOUND` | 409 | License is bound to another device |
| `NOT_BOUND` | 409 | License is not bound to any device |
| `HARDWARE_MISMATCH` | 403 | Hardware ID doesn't match bound device |
| **Features & Quotas** |||
| `FEATURE_NOT_INCLUDED` | 403 | Feature not in license tier |
| `QUOTA_EXCEEDED` | 403 | Usage quota exceeded |
| **Validation** |||
| `INVALID_REQUEST` | 400 | Request payload is invalid |
| `MISSING_FIELD` | 400 | Required field is missing |
| `INVALID_FIELD` | 400 | Field value is invalid |
| **Authentication** |||
| `MISSING_TOKEN` | 401 | No authorization token provided |
| `INVALID_HEADER` | 400 | Authorization header malformed |
| `INVALID_TOKEN` | 401 | Token is invalid |
| `TOKEN_EXPIRED` | 401 | Token has expired |
| `INSUFFICIENT_SCOPE` | 403 | Token lacks required permissions |
| `AUTH_DISABLED` | 501 | Authentication not configured |
| **Server Errors** |||
| `NOT_FOUND` | 404 | Resource not found |
| `CONFLICT` | 409 | Operation conflicts with current state |
| `DATABASE_ERROR` | 500 | Database operation failed |
| `CONFIG_ERROR` | 500 | Server configuration error |
| `CRYPTO_ERROR` | 500 | Encryption operation failed |
| `NETWORK_ERROR` | 502 | External service communication failed |
| `INTERNAL_ERROR` | 500 | Unexpected server error |

---

## Testing

Run all tests:

```sh
cargo test
```

Run with all features enabled:

```sh
cargo test --features "admin-api,jwt-auth"
```

Run with logging:

```sh
RUST_LOG=info cargo test
```

---

## Roadmap

See the full [ROADMAP.md](docs/public/ROADMAP.md) for detailed development plans.

**Current Status: Phase 12 In Progress**

- Activation/validation/deactivation
- Heartbeat mechanism
- Hardware binding
- Encrypted storage
- Configuration system
- Configurable license key generation
- Tier-based feature system
- JWT authentication middleware
- Admin API (CRUD operations)
- Rate limiting middleware
- Feature gating (tier-based access control)
- License lifecycle management (suspend, revoke, reinstate, extend)
- Usage tracking endpoints
- Background jobs (grace period expiration, license expiration, stale device cleanup)
- License blacklisting (permanent ban with audit trail)
- Request validation utilities (UUID, license key, hardware ID, datetime formats)
- OpenAPI 3.0 specification with Swagger UI
- Standardized error response format
- Logging & observability (request ID tracking, health check, structured license event logging)
- Updated client library with new v1 API methods (bind/release/validate/validate-feature/heartbeat)
- Secure encrypted cache for offline/air-gapped system support
- Grace period support for air-gapped systems
- Working code examples (basic-client, air-gapped, feature-gating)
- **Admin API IP whitelisting** (CIDR support, IPv4/IPv6, proxy header support)

**Upcoming:**

- Audit logging
- API key rotation
- Webhook notifications
- Dashboard UI
- Analytics and reporting

## Documentation

Comprehensive documentation is available in the `docs/` folder:

### Guides

| Guide | Description |
|-------|-------------|
| [Getting Started](docs/guide/getting-started.md) | 5-minute quickstart with feature flags and setup |
| [Client Integration](docs/guide/client-integration.md) | Full client lifecycle, offline validation, feature gating |
| [Server Deployment](docs/guide/server-deployment.md) | Database setup, Docker, nginx/traefik, production checklist |
| [Admin API](docs/guide/admin-api.md) | All admin endpoints, authentication, security best practices |
| [Troubleshooting](docs/guide/troubleshooting.md) | Common errors, debugging tips, FAQ |

### API Reference

| Resource | Description |
|----------|-------------|
| [REST API Reference](docs/api/rest-api.md) | Complete endpoint documentation with examples |
| [OpenAPI Spec](docs/api/openapi.json) | OpenAPI 3.1.0 specification |
| `/swagger-ui` | Interactive API explorer (when running with `openapi` feature) |

### Examples

Complete, runnable examples are available in `docs/examples/`:

| Example | Description |
|---------|-------------|
| [basic-client](docs/examples/basic-client/) | Minimal client integration with runtime license key entry |
| [air-gapped](docs/examples/air-gapped/) | Offline validation with encrypted cache and grace periods |
| [feature-gating](docs/examples/feature-gating/) | Enable/disable features based on license tier |

Each example includes a README with step-by-step instructions for both Windows (PowerShell) and Mac/Linux (bash).

---

## Contributing

PRs, issues, and discussions are all welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## License

MIT License — see [LICENSE](LICENSE) for details.
