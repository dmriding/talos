# Talos Roadmap

This roadmap outlines all tasks to evolve Talos into a full-featured, production-ready licensing library. Tasks are organized by priority and phase, with dependencies noted.

**Design Philosophy:** Talos is an **open-source, generic licensing library**. All features should be:
- **Configurable** - No hardcoded values; users configure for their needs
- **Optional** - Advanced features are opt-in, not required
- **Generic** - No product-specific naming or assumptions in the core library
- **Backwards Compatible** - Existing simple use cases continue to work

---

## Current State Summary

**What Talos Has Today:**
- Basic license activation/validation/deactivation
- Heartbeat mechanism
- Hardware binding (SHA-256 fingerprint)
- AES-256-GCM encrypted local storage
- SQLite and PostgreSQL database support
- Simple REST API (no authentication)
- Cross-platform client library

**What a Production Licensing System Needs:**
- Full Admin API for management integrations
- Optional JWT service authentication
- Configurable license key generation (`PREFIX-XXXX-XXXX-XXXX`)
- Optional organization/tenant grouping (1 org = N licenses)
- Hardware bind/release workflow
- Feature gating and validation
- Optional quota/usage tracking
- Configurable tier system
- Grace period and revocation handling
- Background jobs for expiration

---

## Phase 0: Library Architecture (P0 - Critical)

### 0.1 Cargo Feature Flags

Structure the library so advanced features are opt-in:

- [x] Define feature flags in `Cargo.toml`:
  - `default = ["server", "sqlite"]` - Basic server functionality with SQLite
  - `server` - Server components (handlers, database)
  - `sqlite` - SQLite database backend
  - `postgres` - PostgreSQL database backend
  - *(reserved for future)* `jwt-auth`, `admin-api`, `background-jobs`, `quota-tracking`
- [x] Gate code behind `#[cfg(feature = "...")]` attributes
- [x] Document feature combinations in README
- [x] Ensure `cargo build` works with minimal features

### 0.2 Configuration System Enhancement

- [x] Extend `config.rs` to support all new options
- [x] All values should have sensible defaults
- [x] Support environment variable overrides for all config
- [x] Add configuration validation on startup
- [x] Document all configuration options

```toml
# Example config.toml showing all options with defaults
[server]
host = "0.0.0.0"
port = 8080

[license]
key_prefix = "LIC"              # Configurable prefix (default: "LIC")
key_segments = 4                # Number of segments (default: 4)
key_segment_length = 4          # Characters per segment (default: 4)

[database]
url = "sqlite://talos.db"       # SQLite or PostgreSQL

[auth]                          # Optional: requires "jwt-auth" feature
enabled = false
jwt_secret = "env:TALOS_JWT_SECRET"
jwt_issuer = "talos"
jwt_audience = "talos-api"

[jobs]                          # Optional: requires "background-jobs" feature
enabled = false
expiration_check_interval = "1h"
grace_period_check_interval = "1h"

[quota]                         # Optional: requires "quota-tracking" feature
enabled = false
```

---

## Phase 1: Core Admin API (P0 - Critical)

### 1.1 Database Schema Migration ✅

- [x] Create new migration file for extended schema
- [x] Add `org_id` and `org_name` columns to licenses table (nullable for simple use cases)
- [x] Add `license_key` column (configurable `PREFIX-XXXX-XXXX-XXXX` format)
- [x] Add `tier` column (nullable, optional for users who don't need tiers)
- [ ] Change `features` from TEXT to JSONB array *(deferred - current TEXT format works)*
- [x] Add hardware binding fields (`hardware_id`, `device_name`, `device_info`, `bound_at`, `last_seen_at`)
- [x] Add status lifecycle fields (`status`, `suspended_at`, `revoked_at`, `revoke_reason`, `grace_period_ends_at`, `suspension_message`)
- [ ] Add bandwidth quota fields - **gated behind `quota-tracking` feature** (`bandwidth_used_bytes`, `bandwidth_limit_bytes`, `quota_exceeded`, `quota_restricted_features`) *(deferred to quota feature)*
- [x] Add blacklist fields (`is_blacklisted`, `blacklisted_at`, `blacklist_reason`)
- [x] Add `metadata` JSONB column for arbitrary user data (Stripe IDs, custom fields, etc.)
- [x] Create indexes on `org_id`, `license_key`, `hardware_id`, `status`, `expires_at`
- [x] Create `license_binding_history` table for audit trail (optional, can be disabled)
- [ ] Create `api_tokens` table for service authentication - **gated behind `jwt-auth` feature** *(deferred to jwt-auth feature)*
- [ ] Write migration rollback script *(deferred)*
- [x] Update `License` struct with all new fields
- [x] Update `insert_license` to handle all 26 fields
- [x] Add database methods: `get_license_by_key`, `license_key_exists`, `list_licenses_by_org`, `update_license_status`, `bind_license`, `release_license`, `record_binding_history`, `update_last_seen`
- [x] Add helper methods: `License::is_bound()`, `is_expired()`, `is_in_grace_period()`, `is_valid()`
- [x] Add `LicenseBindingHistory` struct and `BindingAction`/`PerformedBy` enums
- [x] Write tests for all new database methods (12 tests)

### 1.2 License Key Generation

- [ ] Create `src/license_key.rs` module
- [ ] Implement character set (excluding ambiguous: 0, O, I, L, 1)
- [ ] Implement `generate_license_key()` function with cryptographic randomness
- [ ] Add configurable prefix via config (default: "LIC")
- [ ] Add configurable segment count and length via config
- [ ] Add key format validation function
- [ ] Add collision detection (check DB before returning)
- [ ] Write unit tests for key generation uniqueness
- [ ] Write unit tests for key format validation

```rust
// Example usage - prefix is configurable, not hardcoded
let config = LicenseKeyConfig {
    prefix: "KERYX".to_string(),  // User configures this
    segments: 4,
    segment_length: 4,
};
let key = generate_license_key(&config); // "KERYX-A1B2-C3D4-E5F6-G7H8"
```

### 1.3 JWT Authentication Middleware

**Gated behind `jwt-auth` feature flag**

- [ ] Add `jsonwebtoken` crate as optional dependency
- [ ] Create `src/server/auth.rs` module
- [ ] Implement JWT validation middleware for Axum
- [ ] Support HS256 algorithm with shared secret
- [ ] Validate `sub`, `iat`, `exp` claims
- [ ] Implement scope-based authorization (`licenses:read`, `licenses:write`, `licenses:*`)
- [ ] Add `TALOS_JWT_SECRET` environment variable
- [ ] Add `TALOS_JWT_ISSUER` and `TALOS_JWT_AUDIENCE` config options
- [ ] Create extractor for authenticated requests
- [ ] **When feature disabled**: Admin endpoints return 501 Not Implemented or are simply not mounted
- [ ] Write unit tests for JWT validation
- [ ] Write integration tests for protected endpoints

### 1.4 Tier Configuration System

**Optional feature** - users can ignore tiers entirely if they don't need them.

- [ ] Create `src/tiers.rs` module
- [ ] Define `TierConfig` struct with `name`, `features`, `bandwidth_gb` (all optional)
- [ ] Allow tiers to be defined via config file (not hardcoded)
- [ ] Provide sensible example tiers in documentation
- [ ] Implement `get_tier_config(tier_name)` function
- [ ] Implement `get_bandwidth_limit_bytes(tier_name)` function (returns None if quota-tracking disabled)
- [ ] Implement `get_tier_features(tier_name)` function
- [ ] **Tiers are optional**: If no tier specified on license, skip tier-based logic
- [ ] Write unit tests for tier lookups

```toml
# Example config - users define their own tiers
[tiers.free]
features = []
bandwidth_gb = 0

[tiers.pro]
features = ["feature_a", "feature_b"]
bandwidth_gb = 500

[tiers.enterprise]
features = ["feature_a", "feature_b", "feature_c"]
bandwidth_gb = 0  # unlimited
```

### 1.5 Admin API Endpoints

**Gated behind `admin-api` feature flag** - Without this feature, only client endpoints are available.

#### Create License
- [ ] Implement `POST /api/v1/licenses` handler
- [ ] Accept `org_id`, `org_name`, `tier`, `features`, `expires_at`, `metadata` (all optional except features)
- [ ] Generate license key automatically using configured prefix
- [ ] Derive limits from tier configuration (if tier provided and tiers configured)
- [ ] Return full license object with generated `license_id` and `license_key`
- [ ] Add JWT authentication requirement (if `jwt-auth` feature enabled)
- [ ] Write integration tests

#### Batch Create Licenses
- [ ] Implement `POST /api/v1/licenses/batch` handler
- [ ] Accept `count` parameter for number of licenses
- [ ] Generate unique keys for each license
- [ ] Use database transaction for atomicity
- [ ] Return array of created license summaries
- [ ] Write integration tests

#### Get License
- [ ] Implement `GET /api/v1/licenses/{license_id}` handler
- [ ] Return full license object with all fields
- [ ] Include computed `is_bound` field
- [ ] Add JWT authentication requirement
- [ ] Write integration tests

#### List Organization Licenses
- [ ] Implement `GET /api/v1/licenses?org_id={id}` handler
- [ ] Return paginated list of licenses for org
- [ ] Include summary stats (`total_licenses`, `bound_licenses`)
- [ ] Add JWT authentication requirement
- [ ] Write integration tests

#### Update License
- [ ] Implement `PATCH /api/v1/licenses/{license_id}` handler
- [ ] Support updating `tier`, `features`, `expires_at`
- [ ] Re-derive limits when tier changes
- [ ] Return updated license object
- [ ] Add JWT authentication requirement
- [ ] Write integration tests

---

## Phase 2: Device Management (P0 - Critical)

### 2.1 Hardware Binding System

#### Client Bind Endpoint
- [ ] Implement `POST /api/v1/client/bind` handler
- [ ] Accept `license_key`, `hardware_id`, `device_name`, `device_info`
- [ ] Check if license exists and is valid
- [ ] Check if license is already bound to different hardware
- [ ] Set `hardware_id`, `device_name`, `device_info`, `bound_at`
- [ ] Record binding in `license_binding_history`
- [ ] Return success with license details or error with bound device name
- [ ] No authentication required (rate-limited)
- [ ] Write integration tests

#### Client Release Endpoint
- [ ] Implement `POST /api/v1/client/release` handler
- [ ] Accept `license_key`, `hardware_id`
- [ ] Verify `hardware_id` matches current binding
- [ ] Clear hardware binding fields
- [ ] Record release in `license_binding_history`
- [ ] Return success confirmation
- [ ] No authentication required (rate-limited)
- [ ] Write integration tests

#### Admin Force Release Endpoint
- [ ] Implement `POST /api/v1/licenses/{license_id}/release` handler
- [ ] Accept `reason` parameter
- [ ] Force unbind regardless of hardware_id
- [ ] Record admin release in `license_binding_history` with `performed_by: "admin"`
- [ ] Return previous binding details
- [ ] Add JWT authentication requirement
- [ ] Write integration tests

### 2.2 Updated Validation Flow

#### Client Validate Endpoint
- [ ] Implement `POST /api/v1/client/validate` handler
- [ ] Accept `license_key`, `hardware_id`
- [ ] Check license exists
- [ ] Check license not expired
- [ ] Check license not revoked/suspended (handle grace period)
- [ ] Check license is bound
- [ ] Check hardware_id matches binding
- [ ] Check not blacklisted
- [ ] Update `last_seen_at` timestamp
- [ ] Return validation result with features and tier
- [ ] Return appropriate error codes for each failure case
- [ ] No authentication required (rate-limited)
- [ ] Write integration tests for all validation paths

#### Validate-or-Bind Convenience Endpoint
- [ ] Implement `POST /api/v1/client/validate-or-bind` handler
- [ ] If bound to this hardware: validate and return
- [ ] If unbound: bind first, then validate
- [ ] If bound to other hardware: return ALREADY_BOUND error
- [ ] Write integration tests

### 2.3 Updated Heartbeat

- [ ] Update `POST /api/v1/client/heartbeat` to use `license_key`
- [ ] Verify hardware_id matches binding
- [ ] Update `last_seen_at` timestamp
- [ ] Return server timestamp
- [ ] Write integration tests

---

## Phase 3: Feature Gating (P0 - Critical)

### 3.1 Feature Validation Endpoint

- [ ] Implement `POST /api/v1/client/validate-feature` handler
- [ ] Accept `license_key`, `hardware_id`, `feature`
- [ ] Perform full license validation first
- [ ] Check if feature is in license's feature list
- [ ] Check if feature is in `quota_restricted_features`
- [ ] Return `allowed: true/false` with appropriate message
- [ ] Return specific error codes: `FEATURE_NOT_INCLUDED`, `QUOTA_EXCEEDED`
- [ ] Write integration tests

### 3.2 Quota Enforcement

- [ ] Implement quota checking in feature validation
- [ ] When `quota_exceeded = true`, add "relay" to restricted features
- [ ] Return user-friendly messages about quota status
- [ ] Write integration tests for quota scenarios

---

## Phase 4: Lifecycle Management (P1 - High)

### 4.1 Revoke License

- [ ] Implement `POST /api/v1/licenses/{license_id}/revoke` handler
- [ ] Accept `reason`, `grace_period_days`, `message`
- [ ] If `grace_period_days = 0`: set status to 'revoked' immediately
- [ ] If `grace_period_days > 0`: set status to 'suspended', calculate `grace_period_ends_at`
- [ ] Store `revoke_reason` and `suspension_message`
- [ ] Add JWT authentication requirement
- [ ] Write integration tests

### 4.2 Reinstate License

- [ ] Implement `POST /api/v1/licenses/{license_id}/reinstate` handler
- [ ] Accept `new_expires_at`, `reset_bandwidth`
- [ ] Set status back to 'active'
- [ ] Clear suspension fields
- [ ] Optionally reset bandwidth counters
- [ ] Add JWT authentication requirement
- [ ] Write integration tests

### 4.3 Extend License

- [ ] Implement `POST /api/v1/licenses/{license_id}/extend` handler
- [ ] Accept `new_expires_at`, `reset_bandwidth`
- [ ] Update `expires_at`
- [ ] Optionally reset bandwidth counters
- [ ] Add JWT authentication requirement
- [ ] Write integration tests

### 4.4 Update Usage

- [ ] Implement `PATCH /api/v1/licenses/{license_id}/usage` handler
- [ ] Accept `bandwidth_used_bytes`, `bandwidth_limit_bytes`
- [ ] Update usage fields
- [ ] Calculate and set `quota_exceeded` flag
- [ ] Set `quota_restricted_features` when exceeded
- [ ] Add JWT authentication requirement
- [ ] Write integration tests

---

## Phase 5: Background Jobs (P1 - High)

### 5.1 Job Infrastructure

- [ ] Add `tokio-cron-scheduler` or similar crate
- [ ] Create `src/jobs/mod.rs` module
- [ ] Implement job runner with configurable schedules
- [ ] Add job logging and error handling
- [ ] Add configuration for job intervals

### 5.2 Grace Period Expiration Job

- [ ] Create `src/jobs/grace_period.rs`
- [ ] Query licenses where `status = 'suspended'` AND `grace_period_ends_at < NOW()`
- [ ] Update status to 'revoked'
- [ ] Set `revoked_at` timestamp
- [ ] Log affected licenses
- [ ] Schedule to run every hour
- [ ] Write integration tests

### 5.3 License Expiration Job

- [ ] Create `src/jobs/expiration.rs`
- [ ] Query licenses where `status = 'active'` AND `expires_at < NOW()`
- [ ] Update status to 'expired'
- [ ] Log affected licenses
- [ ] Schedule to run every hour
- [ ] Write integration tests

### 5.4 Stale Device Cleanup Job (Optional)

- [ ] Create `src/jobs/stale_devices.rs`
- [ ] Query licenses where `last_seen_at < NOW() - 90 days`
- [ ] Clear hardware binding (auto-release)
- [ ] Record in binding history with `performed_by: "system"`
- [ ] Schedule to run daily
- [ ] Make configurable (enable/disable, threshold days)
- [ ] Write integration tests

---

## Phase 6: Blacklist & Security (P2 - Medium)

### 6.1 Blacklist Endpoint

- [ ] Implement `POST /api/v1/licenses/{license_id}/blacklist` handler
- [ ] Accept `reason`, `message`
- [ ] Set `is_blacklisted = true`
- [ ] Set status to 'revoked'
- [ ] Store blacklist reason and timestamp
- [ ] Add JWT authentication requirement
- [ ] Write integration tests

### 6.2 Rate Limiting

- [ ] Add `tower-governor` or similar crate
- [ ] Implement rate limiting middleware
- [ ] Configure limits per endpoint:
  - `/validate`: 100/minute per IP
  - `/heartbeat`: 60/minute per IP
  - `/bind`, `/release`: 10/minute per IP
- [ ] Add configuration options for limits
- [ ] Return 429 Too Many Requests with retry-after header
- [ ] Write integration tests

### 6.3 Request Validation

- [ ] Add input validation for all endpoints
- [ ] Validate UUID formats
- [ ] Validate license key format
- [ ] Validate hardware_id format (SHA-256 hex)
- [ ] Return 400 Bad Request with specific error messages
- [ ] Write validation unit tests

---

## Phase 7: API Documentation & Polish (P2 - Medium)

### 7.1 OpenAPI Specification

- [ ] Add `utoipa` crate for OpenAPI generation
- [ ] Document all endpoints with request/response schemas
- [ ] Document error responses and codes
- [ ] Document authentication requirements
- [ ] Generate OpenAPI JSON/YAML
- [ ] Add Swagger UI endpoint for interactive docs

### 7.2 Error Response Standardization

- [ ] Create standardized error response format
- [ ] Implement error response builder
- [ ] Map all error types to HTTP status codes
- [ ] Include error codes in all responses
- [ ] Document all error codes

### 7.3 Logging & Observability

- [ ] Add structured logging with `tracing`
- [ ] Log all API requests with timing
- [ ] Log all license state changes
- [ ] Add request ID tracking
- [ ] Add health check endpoint (`GET /health`)
- [ ] Add metrics endpoint (optional)

---

## Phase 8: Client Library Updates (P1 - High)

### 8.1 Update Client Struct

- [ ] Update `License` struct to use `license_key` instead of `license_id`
- [ ] Add `tier` and `features` fields
- [ ] Add `org_name` field for display
- [ ] Update serialization/deserialization

### 8.2 New Client Methods

- [ ] Implement `bind()` method
- [ ] Implement `release()` method
- [ ] Implement `validate_feature(feature: &str)` method
- [ ] Update `validate()` to use new endpoint
- [ ] Update `heartbeat()` to use new endpoint
- [ ] Remove `activate()` (replaced by `bind()`)
- [ ] Remove `deactivate()` (replaced by `release()`)

### 8.3 Error Handling Updates

- [ ] Add new error variants for all error codes
- [ ] Implement user-friendly error messages
- [ ] Handle grace period warnings in validation response

---

## Phase 9: Testing (Ongoing)

### 9.1 Unit Tests

- [ ] License key generation uniqueness (1000+ keys)
- [ ] License key format validation
- [ ] JWT token creation and validation
- [ ] Tier configuration lookups
- [ ] Feature permission logic
- [ ] Quota calculation logic
- [ ] Error code mapping

### 9.2 Integration Tests

- [ ] Full license lifecycle (create -> bind -> validate -> release)
- [ ] Multi-license organization flow
- [ ] Bind/release workflow
- [ ] Grace period flow (suspend -> grace -> revoke)
- [ ] Quota exceeded flow
- [ ] Tier upgrade/downgrade
- [ ] Blacklist behavior
- [ ] Background job execution

### 9.3 Load Tests

- [ ] Validation endpoint: target 1000 req/s
- [ ] Heartbeat endpoint: target 500 req/s
- [ ] Document performance baselines
- [ ] Identify and address bottlenecks

---

## Phase 10: Deployment & Operations (P1 - High)

### 10.1 Configuration

- [ ] Document all environment variables
- [ ] Create example `.env` file
- [ ] Create example `config.toml` for Keryx deployment
- [ ] Add configuration validation on startup

### 10.2 Docker

- [ ] Create optimized Dockerfile
- [ ] Create docker-compose.yml with PostgreSQL
- [ ] Document container deployment
- [ ] Add health check to container

### 10.3 Database Migrations

- [ ] Document migration process
- [ ] Create migration scripts for existing Talos deployments
- [ ] Test migration from current schema to Keryx schema

---

## Dependency Graph

```
Phase 1.1 (Schema) ──────────────────────────────────────┐
                                                         │
Phase 1.2 (Key Gen) ─────────────────────────────────────┤
                                                         │
Phase 1.3 (JWT Auth) ────────────────────────────────────┤
                                                         │
Phase 1.4 (Tiers) ───────────────────────────────────────┤
                                                         ▼
Phase 1.5 (Admin API) ◄──────────────────────────────────┘
         │
         ▼
Phase 2 (Device Management) ──────────────────────────────┐
         │                                                │
         ▼                                                │
Phase 3 (Feature Gating) ─────────────────────────────────┤
         │                                                │
         ▼                                                │
Phase 4 (Lifecycle) ──────────────────────────────────────┤
         │                                                │
         ▼                                                │
Phase 5 (Background Jobs) ────────────────────────────────┤
         │                                                │
         ▼                                                │
Phase 6 (Security) ◄──────────────────────────────────────┘
         │
         ▼
Phase 7 (Documentation)
         │
         ▼
Phase 8 (Client Updates)
         │
         ▼
Phase 9 (Testing) ────► Phase 10 (Deployment)
```

---

## Quick Reference: New Endpoints

### Admin Endpoints (JWT Required)

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/licenses` | Create single license |
| POST | `/api/v1/licenses/batch` | Create multiple licenses |
| GET | `/api/v1/licenses?org_id={id}` | List org's licenses |
| GET | `/api/v1/licenses/{license_id}` | Get license details |
| PATCH | `/api/v1/licenses/{license_id}` | Update tier/features |
| POST | `/api/v1/licenses/{license_id}/revoke` | Suspend/revoke |
| POST | `/api/v1/licenses/{license_id}/reinstate` | Reinstate |
| POST | `/api/v1/licenses/{license_id}/extend` | Extend expiry |
| POST | `/api/v1/licenses/{license_id}/release` | Admin force unbind |
| POST | `/api/v1/licenses/{license_id}/blacklist` | Permanent ban |
| PATCH | `/api/v1/licenses/{license_id}/usage` | Update bandwidth |

### Client Endpoints (Rate-Limited, No Auth)

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/client/bind` | Bind license to hardware |
| POST | `/api/v1/client/release` | Release from hardware |
| POST | `/api/v1/client/validate` | Validate license |
| POST | `/api/v1/client/validate-or-bind` | Validate or auto-bind |
| POST | `/api/v1/client/validate-feature` | Check feature access |
| POST | `/api/v1/client/heartbeat` | Liveness ping |

---

## Estimated Effort by Phase

| Phase | Description | Complexity |
|-------|-------------|------------|
| 1 | Core Admin API | High |
| 2 | Device Management | Medium |
| 3 | Feature Gating | Low |
| 4 | Lifecycle Management | Medium |
| 5 | Background Jobs | Medium |
| 6 | Security | Medium |
| 7 | Documentation | Low |
| 8 | Client Updates | Medium |
| 9 | Testing | High |
| 10 | Deployment | Low |

---

## Notes

### Open Source Design Principles

- **No Product-Specific Code**: The Talos codebase should never contain Keryx-specific naming, constants, or logic. Keryx is just one user of the library.
- **Configuration Over Code**: All customization (key prefix, tiers, features) happens via configuration, not code changes.
- **Feature Flags**: Advanced features are opt-in via Cargo features. A simple use case shouldn't require complex setup.
- **Sensible Defaults**: Works out of the box with SQLite and basic endpoints. Advanced features require explicit opt-in.

### Migration & Compatibility

- **Breaking Changes**: The schema changes in Phase 1.1 are breaking. Existing Talos deployments will need migration.
- **Backwards Compatibility**: Old `/activate`, `/validate`, `/deactivate`, `/heartbeat` endpoints can be deprecated but kept for transition period.
- **Testing Strategy**: Write tests alongside implementation, not after. Each task should include its tests.

### For Keryx Integration Specifically

When deploying Talos for Keryx, the configuration would look like:

```toml
[license]
key_prefix = "KERYX"

[auth]
enabled = true
jwt_secret = "env:TALOS_JWT_SECRET"

[quota]
enabled = true

[tiers.free]
features = []
bandwidth_gb = 0

[tiers.starter]
features = ["relay"]
bandwidth_gb = 500

[tiers.pro]
features = ["relay", "priority_support"]
bandwidth_gb = 2000

[tiers.team]
features = ["relay", "priority_support", "dedicated_relay"]
bandwidth_gb = 10000
```

This is **configuration**, not code. Another user might configure completely different tiers and a different key prefix.