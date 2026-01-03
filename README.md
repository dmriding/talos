# Talos — Secure Rust Licensing System

![Build Status](https://github.com/dmriding/talos/actions/workflows/ci.yml/badge.svg)
![License](https://img.shields.io/badge/license-MIT-blue)
![Rust Version](https://img.shields.io/badge/rust-stable-blue)

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
│   └── public/
│       └── ROADMAP.md            # Development roadmap
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
talos = { git = "https://github.com/dmriding/talos" }
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

### Examples

```toml
# Default: server + SQLite
talos = { git = "https://github.com/dmriding/talos" }

# Client-only (no server components)
talos = { git = "https://github.com/dmriding/talos", default-features = false }

# Server with PostgreSQL instead of SQLite
talos = { git = "https://github.com/dmriding/talos", default-features = false, features = ["server", "postgres"] }

# Server with both SQLite and PostgreSQL
talos = { git = "https://github.com/dmriding/talos", features = ["postgres"] }

# Full server with admin API and JWT auth
talos = { git = "https://github.com/dmriding/talos", features = ["admin-api", "jwt-auth"] }

# Server with background jobs enabled
talos = { git = "https://github.com/dmriding/talos", features = ["background-jobs"] }

# Full-featured server
talos = { git = "https://github.com/dmriding/talos", features = ["admin-api", "jwt-auth", "rate-limiting", "background-jobs"] }
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
use talos::client::license::License;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut license = License {
        license_id: "LICENSE-12345".into(),
        client_id: "".into(), // Auto-filled by activate()
        expiry_date: "2025-12-31".into(),
        features: vec!["feature1".into()],
        server_url: "http://127.0.0.1:8080".into(),
        signature: "dummy".into(),
        is_active: false,
    };

    license.activate().await?;
    println!("License activated.");

    if license.validate().await? {
        println!("License validated.");
    }

    license.heartbeat().await?;
    license.deactivate().await?;

    Ok(())
}
```

Or run the provided example:

```sh
cargo run --example manual_activate
```

---

## Server API Endpoints

### Client Endpoints (always available)

| Method | Endpoint      | Description              |
|--------|---------------|--------------------------|
| POST   | `/activate`   | Activate a license       |
| POST   | `/validate`   | Validate if license is active |
| POST   | `/deactivate` | Deactivate a license     |
| POST   | `/heartbeat`  | Send heartbeat ping      |

### Admin Endpoints (requires `admin-api` feature)

| Method | Endpoint                              | Description                        |
|--------|---------------------------------------|------------------------------------|
| POST   | `/api/v1/licenses`                    | Create a new license               |
| POST   | `/api/v1/licenses/batch`              | Batch create licenses              |
| GET    | `/api/v1/licenses/{id}`               | Get license by ID                  |
| GET    | `/api/v1/licenses?org_id=X`           | List licenses by org               |
| PATCH  | `/api/v1/licenses/{id}`               | Update a license                   |
| POST   | `/api/v1/licenses/{id}/suspend`       | Suspend a license                  |
| POST   | `/api/v1/licenses/{id}/revoke`        | Revoke a license                   |
| POST   | `/api/v1/licenses/{id}/reinstate`     | Reinstate a suspended/revoked license |
| POST   | `/api/v1/licenses/{id}/extend`        | Extend license expiration          |
| POST   | `/api/v1/licenses/{id}/usage`         | Update usage/bandwidth metrics     |
| POST   | `/api/v1/licenses/{id}/validate-feature` | Validate feature access         |
| POST   | `/api/v1/licenses/{id}/release`       | Release hardware binding           |
| POST   | `/api/v1/licenses/{id}/blacklist`     | Permanently blacklist a license    |

All client requests use:

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

**Current Status: Phase 6 Complete**

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

**Upcoming:**

- OpenAPI documentation
- Webhook notifications
- Dashboard UI

---

## Contributing

PRs, issues, and discussions are all welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## License

MIT License — see [LICENSE](LICENSE) for details.
