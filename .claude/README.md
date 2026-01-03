# Talos - AI Assistant Guide

This document provides context for AI assistants (Claude, GPT, Copilot, etc.) helping with this project.

## Project Overview

**Talos** is a secure licensing system for Rust applications. It provides:

- **Server-side** components for license management (database, API handlers, JWT auth)
- **Client-side** components for license validation and heartbeat
- **Flexible configuration** via TOML files and environment variables
- **Feature flags** for modular builds

## Architecture

```
talos/
├── src/
│   ├── lib.rs              # Main library entry point
│   ├── config.rs           # Configuration system (TalosConfig)
│   ├── errors.rs           # Error types (LicenseError)
│   ├── encryption.rs       # AES-GCM encryption utilities
│   ├── hardware.rs         # Hardware fingerprinting
│   ├── license_key.rs      # License key generation/validation
│   ├── tiers.rs            # Tier configuration system
│   ├── client/             # Client-side modules
│   │   ├── license.rs      # License struct and validation
│   │   ├── heartbeat.rs    # Heartbeat mechanism
│   │   ├── encrypted_storage.rs  # Secure license storage
│   │   └── key_generation.rs     # Key pair generation
│   └── server/             # Server-side modules (requires "server" feature)
│       ├── mod.rs          # Module exports
│       ├── database.rs     # Database abstraction (SQLite/PostgreSQL)
│       ├── handlers.rs     # Client-facing API handlers
│       ├── admin.rs        # Admin API handlers (requires "admin-api" feature)
│       ├── auth.rs         # JWT authentication (requires "jwt-auth" feature)
│       ├── routes.rs       # Router builder
│       └── server_sim.rs   # In-memory simulator for tests
├── tests/                  # Integration tests
├── docs/                   # Documentation
│   └── public/
│       └── ROADMAP.md      # Development roadmap
└── config.toml             # Example configuration
```

## Feature Flags

The project uses Cargo feature flags for modularity:

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `server` | Server components (handlers, database) | axum, sqlx, tower |
| `sqlite` | SQLite database backend | sqlx/sqlite |
| `postgres` | PostgreSQL database backend | sqlx/postgres |
| `jwt-auth` | JWT authentication middleware | jsonwebtoken |
| `admin-api` | Admin CRUD API endpoints | server |

**Default features:** `server`, `sqlite`

## Key Types

### Configuration
- `TalosConfig` - Root configuration struct
- `ServerConfig` - Host, port, heartbeat interval
- `DatabaseConfig` - Database type and URLs
- `AuthConfig` - JWT settings (secret, issuer, audience)
- `LicenseConfig` - Key generation settings

### Database
- `License` - Main license record with all fields
- `Database` - Enum over SQLite/PostgreSQL pools
- `BindingAction`, `PerformedBy` - Audit trail enums

### Authentication
- `Claims` - JWT claims with scope checking
- `AuthenticatedUser` - Axum extractor for protected routes
- `JwtValidator` - Token creation and validation

### Tiers
- `TierConfig` - Features and bandwidth limits
- `Tier` - Wrapper with name and helper methods

## Testing Requirements

**CRITICAL: ALL CODE MUST PASS BEFORE COMMITTING:**

1. **All tests must pass** - No exceptions
2. **Code must be formatted** - `cargo fmt` must produce no changes
3. **CI must pass** - GitHub Actions runs tests on all PRs

### Required Commands Before Every Commit

```bash
# 1. Run all tests with all features
cargo test --all-features

# 2. Check formatting (must produce no output)
cargo fmt --check

# 3. Run clippy with no warnings
cargo clippy --all-features

# If any of these fail, fix the issues before committing!
```

### Optional: Run Specific Tests

```bash
# Run specific test files
cargo test --test admin_api_tests --features admin-api
cargo test --test database_tests

# Run with verbose output
cargo test --all-features -- --nocapture
```

### CI Pipeline

The GitHub Actions workflow runs on every PR and push:
- `cargo test --all-features`
- `cargo fmt --check`
- `cargo clippy --all-features`

**PRs will not be merged if CI fails.**

### For AI Agents

**IMPORTANT:** AI agents (Claude, GPT, etc.) MUST run the following commands and verify they pass before completing any task:

```bash
cargo test --all-features
cargo fmt --check
cargo clippy --all-features
```

**Do not submit results if any of these commands fail.** Fix the issues first.

## Code Style Guidelines

1. **Error Handling**
   - Use `LicenseResult<T>` for fallible operations
   - Return appropriate `LicenseError` variants
   - Map database errors to `ServerError`

2. **Feature Gates**
   - Use `#[cfg(feature = "...")]` for optional modules
   - Keep feature dependencies minimal

3. **Documentation**
   - Add doc comments (`///`) to public items
   - Include examples for complex APIs
   - Use `//!` for module-level docs

4. **Testing**
   - Unit tests in `#[cfg(test)]` blocks within source files
   - Integration tests in `tests/` directory
   - Use `server_sim` for handler tests

## Security Considerations

- **Never commit secrets** (JWT secrets, database credentials)
- **Validate all inputs** at API boundaries
- **Use constant-time comparison** for secrets
- **Encrypt sensitive data** at rest
- **Log security events** (auth failures, suspicious activity)

## Current Development Status

Check `docs/public/ROADMAP.md` for:
- Completed phases (marked with check marks)
- Current work in progress
- Planned features

## Common Tasks

### Adding a New Endpoint

1. Add handler function in `src/server/handlers.rs` or `src/server/admin.rs`
2. Add route in `src/server/routes.rs`
3. Add request/response types
4. Write integration tests in `tests/`
5. Update documentation

### Adding a New Feature Flag

1. Add feature to `Cargo.toml` `[features]` section
2. Gate code with `#[cfg(feature = "...")]`
3. Update `src/server/mod.rs` exports
4. Add tests that run with the feature
5. Document the feature

### Modifying Database Schema

1. Update `License` struct in `src/server/database.rs`
2. Update SQL queries in database methods
3. Update test fixtures in `tests/`
4. Run all database tests

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `TALOS_SERVER_HOST` | Server bind address | 127.0.0.1 |
| `TALOS_SERVER_PORT` | Server port | 8080 |
| `TALOS_DATABASE_TYPE` | Database type (sqlite/postgres) | sqlite |
| `TALOS_DATABASE_URL` | Database connection URL | sqlite://talos.db |
| `TALOS_JWT_SECRET` | JWT signing secret | (required if auth enabled) |
| `TALOS_JWT_ISSUER` | JWT issuer claim | talos |
| `TALOS_JWT_AUDIENCE` | JWT audience claim | talos-api |

## Getting Help

- **Code questions:** Read the module doc comments
- **Architecture:** Check this file and ROADMAP.md
- **Tests failing:** Run with `RUST_BACKTRACE=1` for traces
- **Feature questions:** Check Cargo.toml features section