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
  - _(reserved for future)_ `jwt-auth`, `admin-api`, `background-jobs`, `quota-tracking`
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
- [ ] Change `features` from TEXT to JSONB array _(deferred - current TEXT format works)_
- [x] Add hardware binding fields (`hardware_id`, `device_name`, `device_info`, `bound_at`, `last_seen_at`)
- [x] Add status lifecycle fields (`status`, `suspended_at`, `revoked_at`, `revoke_reason`, `grace_period_ends_at`, `suspension_message`)
- [ ] Add bandwidth quota fields - **gated behind `quota-tracking` feature** (`bandwidth_used_bytes`, `bandwidth_limit_bytes`, `quota_exceeded`, `quota_restricted_features`) _(deferred to quota feature)_
- [x] Add blacklist fields (`is_blacklisted`, `blacklisted_at`, `blacklist_reason`)
- [x] Add `metadata` JSONB column for arbitrary user data (Stripe IDs, custom fields, etc.)
- [x] Create indexes on `org_id`, `license_key`, `hardware_id`, `status`, `expires_at`
- [x] Create `license_binding_history` table for audit trail (optional, can be disabled)
- [ ] Create `api_tokens` table for service authentication - **gated behind `jwt-auth` feature** _(deferred to jwt-auth feature)_
- [ ] Write migration rollback script _(deferred)_
- [x] Update `License` struct with all new fields
- [x] Update `insert_license` to handle all 26 fields
- [x] Add database methods: `get_license_by_key`, `license_key_exists`, `list_licenses_by_org`, `update_license_status`, `bind_license`, `release_license`, `record_binding_history`, `update_last_seen`
- [x] Add helper methods: `License::is_bound()`, `is_expired()`, `is_in_grace_period()`, `is_valid()`
- [x] Add `LicenseBindingHistory` struct and `BindingAction`/`PerformedBy` enums
- [x] Write tests for all new database methods (12 tests)

### 1.2 License Key Generation ✅

- [x] Create `src/license_key.rs` module
- [x] Implement character set (excluding ambiguous: 0, O, I, L, 1)
- [x] Implement `generate_license_key()` function with cryptographic randomness
- [x] Add configurable prefix via config (default: "LIC")
- [x] Add configurable segment count and length via config
- [x] Add key format validation function (`validate_license_key_format()`)
- [x] Add collision detection (`generate_unique_license_key()` with async exists check)
- [x] Write unit tests for key generation uniqueness (12 tests)
- [x] Write unit tests for key format validation
- [x] Add `parse_license_key()` helper function
- [x] Add convenience functions using global config (`generate_license_key_from_config()`, `validate_license_key_format_from_config()`)

```rust
// Example usage - prefix is configurable, not hardcoded
let config = LicenseKeyConfig {
    prefix: "KERYX".to_string(),  // User configures this
    segments: 4,
    segment_length: 4,
};
let key = generate_license_key(&config); // "KERYX-A1B2-C3D4-E5F6-G7H8"
```

### 1.3 JWT Authentication Middleware ✅

**Gated behind `jwt-auth` feature flag**

- [x] Add `jsonwebtoken` crate as optional dependency
- [x] Create `src/server/auth.rs` module
- [x] Implement JWT validation middleware for Axum
- [x] Support HS256 algorithm with shared secret
- [x] Validate `sub`, `iat`, `exp`, `iss`, `aud` claims
- [x] Implement scope-based authorization (`licenses:read`, `licenses:write`, `licenses:*`, wildcard support)
- [x] Add `TALOS_JWT_SECRET` environment variable
- [x] Add `TALOS_JWT_ISSUER`, `TALOS_JWT_AUDIENCE`, `TALOS_TOKEN_EXPIRATION_SECS` config options
- [x] Add `AuthConfig` struct with all JWT settings to config system
- [x] Create `AuthenticatedUser` extractor for authenticated requests
- [x] Create `OptionalUser` extractor for optional authentication
- [x] Create `JwtValidator` for token creation and validation
- [x] Create `AuthState` for middleware state management
- [x] **When feature disabled**: Auth module not compiled, `AuthError::AuthDisabled` returned
- [x] Write unit tests for JWT validation (11 tests)
- [ ] Write integration tests for protected endpoints _(deferred to Phase 1.5 when endpoints exist)_

### 1.4 Tier Configuration System ✅

**Optional feature** - users can ignore tiers entirely if they don't need them.

- [x] Create `src/tiers.rs` module
- [x] Define `TierConfig` struct with `features`, `bandwidth_gb` fields
- [x] Define `Tier` wrapper struct with `name` and helper methods
- [x] Allow tiers to be defined via config file (not hardcoded)
- [x] Add `tiers: HashMap<String, TierConfig>` to `TalosConfig`
- [x] Implement `get_tier_config(tier_name)` function
- [x] Implement `get_bandwidth_limit_bytes(tier_name)` function (returns None if unlimited or missing)
- [x] Implement `get_tier_features(tier_name)` function
- [x] Implement helper functions: `tier_exists()`, `tier_has_feature()`, `get_all_tier_names()`, `get_all_tiers()`
- [x] **Tiers are optional**: If no tier specified on license, functions return None/empty
- [x] Write unit tests for tier lookups (4 tests)

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

### 1.5 Admin API Endpoints ✅

**Gated behind `admin-api` feature flag** - Without this feature, only client endpoints are available.

#### Create License ✅

- [x] Implement `POST /api/v1/licenses` handler
- [x] Accept `org_id`, `org_name`, `tier`, `features`, `expires_at`, `metadata` (all optional)
- [x] Generate license key automatically using configured prefix
- [x] Derive features from tier configuration (if tier provided and tiers configured)
- [x] Return full license object with generated `license_id` and `license_key`
- [x] Write integration tests (14 tests in `tests/admin_api_tests.rs`)

#### Batch Create Licenses ✅

- [x] Implement `POST /api/v1/licenses/batch` handler
- [x] Accept `count` parameter for number of licenses (max 1000)
- [x] Generate unique keys for each license
- [x] Return array of created license summaries
- [x] Write integration tests

#### Get License ✅

- [x] Implement `GET /api/v1/licenses/{license_id}` handler
- [x] Return full license object with all fields
- [x] Include computed `is_bound` field
- [x] Write integration tests

#### List Organization Licenses ✅

- [x] Implement `GET /api/v1/licenses?org_id={id}` handler
- [x] Return paginated list of licenses for org (with `page`, `per_page` params)
- [x] Include `total`, `total_pages` in response
- [x] Write integration tests

#### Update License ✅

- [x] Implement `PATCH /api/v1/licenses/{license_id}` handler
- [x] Support updating `tier`, `features`, `expires_at`, `metadata`
- [x] Re-derive features when tier changes
- [x] Return updated license object
- [x] Write integration tests

**Note:** JWT authentication guard integration deferred to when both `admin-api` and `jwt-auth` features are enabled together. Routes are in place and ready for middleware.

**Protect public endpoints from brute force attacks**

## Phase 2: Device Management (P0 - Critical) ✅

### 2.1 Rate Limiting ✅

**Protect public endpoints from brute force attacks**

- [x] Add `tower-governor` crate as optional dependency (`rate-limiting` feature)
- [x] Add `governor` crate for rate limiting primitives
- [x] Create `src/server/rate_limit.rs` module
- [x] Implement rate limiting middleware for Axum using `SmartIpKeyExtractor`
- [x] Configure limits per endpoint type via `RateLimitConfig`:
  - `/validate`: 100/minute per IP (configurable)
  - `/heartbeat`: 60/minute per IP (configurable)
  - `/bind`, `/release`: 10/minute per IP (configurable)
- [x] Add configuration options in `TalosConfig.rate_limit`
- [x] Return 429 Too Many Requests with `Retry-After` header
- [x] Write unit tests for rate limiting (6 tests)

### 2.2 Hardware Binding System ✅

#### Client Bind Endpoint ✅

- [x] Implement `POST /api/v1/client/bind` handler
- [x] Accept `license_key`, `hardware_id`, `device_name`, `device_info`
- [x] Check if license exists and is valid (not expired, revoked, suspended, blacklisted)
- [x] Check if license is already bound to different hardware
- [x] Set `hardware_id`, `device_name`, `device_info`, `bound_at`
- [x] Record binding in `license_binding_history`
- [x] Return success with license details (id, features, tier, expires_at) or error with bound device name
- [x] No authentication required (rate-limited via `rate-limiting` feature)
- [x] Write unit tests (5 tests in client_api.rs)

#### Client Release Endpoint ✅

- [x] Implement `POST /api/v1/client/release` handler
- [x] Accept `license_key`, `hardware_id`
- [x] Verify `hardware_id` matches current binding
- [x] Clear hardware binding fields
- [x] Record release in `license_binding_history`
- [x] Return success confirmation
- [x] No authentication required (rate-limited)

#### Admin Force Release Endpoint ✅

- [x] Implement `POST /api/v1/licenses/{license_id}/release` handler
- [x] Accept `reason` parameter for audit trail
- [x] Force unbind regardless of hardware_id
- [x] Record admin release in `license_binding_history` with `performed_by: "admin"`
- [x] Return previous binding details (hardware_id, device_name)
- [x] Add to admin routes (JWT authentication when both features enabled)

### 2.3 Updated Validation Flow ✅

#### Client Validate Endpoint ✅

- [x] Implement `POST /api/v1/client/validate` handler
- [x] Accept `license_key`, `hardware_id`
- [x] Check license exists → `LICENSE_NOT_FOUND`
- [x] Check license not blacklisted → `LICENSE_BLACKLISTED`
- [x] Check license not revoked → `LICENSE_REVOKED`
- [x] Check license not expired → `LICENSE_EXPIRED`
- [x] Check license not suspended (handle grace period with warning) → `LICENSE_SUSPENDED`
- [x] Check license is bound → `NOT_BOUND`
- [x] Check hardware_id matches binding → `HARDWARE_MISMATCH`
- [x] Check status is active → `LICENSE_INACTIVE`
- [x] Update `last_seen_at` timestamp
- [x] Return validation result with features, tier, expires_at, grace_period_ends_at, warning
- [x] Return appropriate `ClientErrorCode` for each failure case
- [x] No authentication required (rate-limited)

#### Validate-or-Bind Convenience Endpoint ✅

- [x] Implement `POST /api/v1/client/validate-or-bind` handler
- [x] If bound to this hardware: validate and return
- [x] If unbound: bind first, then validate
- [x] If bound to other hardware: return `ALREADY_BOUND` error with bound device name

### 2.4 Updated Heartbeat ✅

- [x] Implement `POST /api/v1/client/heartbeat` handler using `license_key`
- [x] Verify license exists and is bound
- [x] Verify `hardware_id` matches binding
- [x] Update `last_seen_at` timestamp
- [x] Return server timestamp in RFC 3339 format

---

## Phase 3: Feature Gating (P0 - Critical) ✅

### 3.1 Feature Validation Endpoint ✅

- [x] Implement `POST /api/v1/client/validate-feature` handler
- [x] Accept `license_key`, `hardware_id`, `feature`
- [x] Perform full license validation first
- [x] Check if feature is in license's feature list
- [x] Check if feature is in tier's features (via `get_tier_config`)
- [x] Return `allowed: true/false` with appropriate message
- [x] Return specific error codes: `FEATURE_NOT_INCLUDED`, `QUOTA_EXCEEDED`
- [x] Write unit tests for new error codes

### 3.2 Quota Enforcement (Infrastructure Ready)

- [x] Add `QuotaExceeded` error code to ClientErrorCode
- [x] Add TODO placeholder for quota checking when database fields are added
- [ ] (Phase 4.4) Add `quota_exceeded`, `quota_restricted_features` fields to database
- [ ] (Phase 4.4) Implement full quota checking with bandwidth tracking

**Note:** Full quota enforcement requires the usage tracking fields from Phase 4.4. The validate-feature endpoint is ready to support quota checking once those fields are added.

---

## Phase 4: Lifecycle Management (P1 - High)

### 4.1 Revoke License ✅

- [x] Implement `POST /api/v1/licenses/{license_id}/revoke` handler
- [x] Accept `reason`, `grace_period_days`, `message`
- [x] If `grace_period_days = 0`: set status to 'revoked' immediately
- [x] If `grace_period_days > 0`: set status to 'suspended', calculate `grace_period_ends_at`
- [x] Store `revoke_reason` and `suspension_message`
- [x] Write integration tests (4 tests)

**Note:** JWT authentication guard integration deferred to when both `admin-api` and `jwt-auth` features are enabled together. Route is in place and ready for middleware.

### 4.2 Reinstate License ✅

- [x] Implement `POST /api/v1/licenses/{license_id}/reinstate` handler
- [x] Accept `new_expires_at`, `reset_bandwidth`, `reason`
- [x] Set status back to 'active'
- [x] Clear suspension fields (`suspended_at`, `revoked_at`, `revoke_reason`, `grace_period_ends_at`, `suspension_message`)
- [x] Optionally reset bandwidth counters (infrastructure ready, no-op until quota fields added)
- [x] Write integration tests (5 tests)

### 4.3 Extend License ✅

- [x] Implement `POST /api/v1/licenses/{license_id}/extend` handler
- [x] Accept `new_expires_at`, `reset_bandwidth`, `reason`
- [x] Update `expires_at`
- [x] Optionally reset bandwidth counters (infrastructure ready, no-op until quota fields added)
- [x] Write integration tests (5 tests)

### 4.4 Update Usage ✅

- [x] Implement `PATCH /api/v1/licenses/{license_id}/usage` handler
- [x] Accept `bandwidth_used_bytes`, `bandwidth_limit_bytes`, `reset`
- [x] Calculate and return `quota_exceeded` flag based on usage vs limit
- [x] Calculate and return `usage_percentage`
- [x] Write integration tests (5 tests)
- [ ] Database persistence (requires `quota-tracking` feature, deferred)

---

## Phase 5: Background Jobs (P1 - High) ✅

### 5.1 Job Infrastructure ✅

- [x] Add `tokio-cron-scheduler` crate as optional dependency (`background-jobs` feature)
- [x] Create `src/jobs/mod.rs` module with `JobScheduler` struct
- [x] Implement job runner with configurable cron schedules via `JobConfig`
- [x] Add job logging with `tracing`
- [x] Add configuration for job intervals
- [x] Add methods to run jobs manually: `run_grace_period_check_now()`, etc.
- [x] Write unit test for default config

### 5.2 Grace Period Expiration Job ✅

- [x] Create `src/jobs/grace_period.rs`
- [x] Query licenses where `status = 'suspended'` AND `grace_period_ends_at < NOW()`
- [x] Update status to 'revoked'
- [x] Set `revoked_at` timestamp
- [x] Log affected licenses
- [x] Configurable schedule (default: every hour)
- [x] Write integration tests (2 tests)

### 5.3 License Expiration Job ✅

- [x] Create `src/jobs/license_expiration.rs`
- [x] Query licenses where `status = 'active'` AND `expires_at < NOW()`
- [x] Update status to 'expired'
- [x] Log affected licenses
- [x] Configurable schedule (default: every hour at minute 15)
- [x] Write integration tests (2 tests)

### 5.4 Stale Device Cleanup Job ✅

- [x] Create `src/jobs/stale_devices.rs`
- [x] Query licenses where `last_seen_at < threshold`
- [x] Clear hardware binding (auto-release)
- [x] Record in binding history with `performed_by: "system"`
- [x] Configurable schedule (default: daily at 3 AM)
- [x] Make configurable (enable/disable, threshold days)
- [x] Write integration tests (2 tests)

---

## Phase 6: Blacklist & Security (P2 - Medium)

### 6.1 Blacklist Endpoint ✅

- [x] Implement `POST /api/v1/licenses/{license_id}/blacklist` handler
- [x] Accept `reason`, `message`
- [x] Set `is_blacklisted = true`
- [x] Set status to 'revoked'
- [x] Store blacklist reason and timestamp
- [x] Clear hardware binding on blacklist
- [x] Prevent reinstating blacklisted licenses
- [x] Validate reason is not empty
- [ ] Add JWT authentication requirement _(deferred to jwt-auth feature integration)_
- [x] Write integration tests (6 tests)

### 6.2 Request Validation ✅

- [x] Create `src/server/validation.rs` module
- [x] Validate UUID formats
- [x] Validate license key format
- [x] Validate hardware_id format (SHA-256 hex)
- [x] Validate datetime formats (ISO 8601)
- [x] Validate feature names and org IDs
- [x] Validate string length and non-empty values
- [x] Return 400 Bad Request with specific error messages via `ValidationError`
- [x] Write validation unit tests (12 tests)
- [x] Add documentation with examples (5 doctests)

---

## Phase 6.5: Authentication & Quota Completion (P1 - High)

### 6.5.1 Quota Persistence ✅

- [x] Add migration for quota fields: `bandwidth_used_bytes`, `bandwidth_limit_bytes`, `quota_exceeded`
- [x] Update `License` struct with quota fields
- [x] Update `update_usage` handler to persist to database
- [x] Update validation to check quota from database
- [x] Write integration tests for quota persistence

### 6.5.2 JWT + Admin API Wiring ✅

- [x] Add JWT authentication guard to admin routes (when both `admin-api` and `jwt-auth` enabled)
- [x] Create `AuthLayer` middleware that injects `AuthState` into request extensions
- [x] Add `auth` field to `AppState` (conditional on `jwt-auth` feature)
- [x] Apply `AuthLayer` to admin routes when both features enabled
- [x] Update all test files to include auth state

### 6.5.3 Token Management System ✅

- [x] Create `api_tokens` table migration:
  - `id`, `name`, `token_hash`, `scopes`, `created_at`, `expires_at`, `last_used_at`, `revoked_at`, `created_by`
- [x] Create `ApiToken` struct and database methods
- [x] Implement token validation (hash lookup, scope check, expiry check)
- [x] Implement `POST /api/v1/tokens` - create new token (returns raw token once)
- [x] Implement `GET /api/v1/tokens` - list tokens (metadata only)
- [x] Implement `DELETE /api/v1/tokens/{id}` - revoke token
- [x] Update `last_used_at` on each authenticated request

### 6.5.4 Bootstrap Flow ✅

- [x] Check `TALOS_BOOTSTRAP_TOKEN` env var on startup
- [x] If set and no tokens exist → create bootstrap token with `*` scope
- [x] If tokens exist → ignore env var (prevent re-bootstrap)
- [x] Log warning when bootstrap token is used
- [x] Add CLI command: `talos token create --name "X" --scopes "Y"`
- [x] CLI outputs raw token to stdout once

---

## Phase 7: API Documentation & Polish (P2 - Medium)

### 7.1 OpenAPI Specification ✅

- [x] Add `utoipa` crate for OpenAPI generation
- [x] Document all endpoints with request/response schemas
- [x] Document error responses and codes
- [x] Document authentication requirements (bearer_auth security scheme)
- [x] Generate OpenAPI JSON (`/api-docs/openapi.json`)
- [x] Add Swagger UI endpoint (`/swagger-ui`)
- [x] Add `openapi` feature flag for optional inclusion

### 7.2 Error Response Standardization ✅

- [x] Create standardized error response format (`ApiError` struct)
- [x] Implement error response builder with convenience methods
- [x] Map all error types to HTTP status codes (`ErrorCode` enum)
- [x] Include error codes in all responses (unified format)
- [x] Document all error codes (README.md)

### 7.3 Logging & Observability ✅

- [x] Add structured logging with `tracing`
- [x] Log all API requests with timing
- [x] Log all license state changes
- [x] Add request ID tracking
- [x] Add health check endpoint (`GET /health`)
- [ ] Add metrics endpoint (optional, deferred)

---

## Phase 8: Client Library Updates (P1 - High) ✅

### 8.1 Client Error Types (`src/client/errors.rs`) ✅

- [x] Create `ClientErrorCode` enum matching server error codes
- [x] Create `ClientApiError` struct with code, message, details
- [x] Add `ClientApiError` variant to `LicenseError`
- [x] Implement `From` trait for deserializing server error responses
- [x] Write unit tests for error parsing (5 tests)

### 8.2 Response Types (`src/client/responses.rs`) ✅

- [x] Create `ValidationResult` struct (features, tier, expires_at, grace_period_ends_at, warning)
- [x] Create `BindResult` struct (license_id, features, tier, expires_at)
- [x] Create `FeatureResult` struct (allowed, message, tier)
- [x] Create `HeartbeatResult` struct (server_time, grace_period_ends_at)
- [x] Create server response parsing types (internal)
- [x] Write unit tests for response parsing (4 tests)

### 8.3 Secure Cached Validation (`src/client/cache.rs`) ✅

**Security Requirements:**
- Encrypted with AES-256-GCM (hardware-bound key)
- Tamper-evident (GCM authentication tag)
- Server-provided grace period (cannot be forged client-side)

- [x] Create `CachedValidation` struct (license_key, hardware_id, features, tier, expires_at, grace_period_ends_at, validated_at)
- [x] Implement secure serialization with existing `encrypted_storage` module
- [x] Ensure hardware binding (encryption key derived from hardware ID with salt)
- [x] Add helper methods: `is_valid_for_offline()`, `is_license_expired()`, `grace_period_remaining()`, `matches_hardware()`, `has_feature()`
- [x] Write tests for tamper detection (8 tests including tampered cache test)

### 8.4 Update License Struct (`src/client/license.rs`) ✅

- [x] Add `license_key` as primary field (use in new API)
- [x] Add `hardware_id` field (set after bind)
- [x] Add `cached: Option<CachedValidation>` for offline validation
- [x] Keep legacy fields (`license_id`, `client_id`, etc.) for backwards compatibility
- [x] Update `save_to_disk()` / `load_from_disk()` for new struct
- [x] Create `License::new(license_key, server_url)` constructor

### 8.5 New Client Methods ✅

- [x] Implement `bind(device_name, device_info) -> BindResult`
- [x] Implement `release() -> ()`
- [x] Implement `validate() -> ValidationResult` (online, updates cache)
- [x] Implement `validate_offline() -> ValidationResult` (checks cached grace period)
- [x] Implement `validate_with_fallback() -> ValidationResult` (online with offline fallback)
- [x] Implement `validate_feature(feature) -> FeatureResult` (always online)
- [x] Implement `heartbeat() -> HeartbeatResult` (updates grace period in cache)

### 8.6 Legacy Method Deprecation ✅

- [x] Mark `activate()` as `#[deprecated]` with note to use `bind()`
- [x] Mark `deactivate()` as `#[deprecated]` with note to use `release()`
- [x] Keep legacy methods functional for backwards compatibility
- [x] Legacy methods continue to use old `/activate`, `/deactivate` endpoints

### 8.7 Module Structure Updates ✅

- [x] Create `src/client/errors.rs`
- [x] Create `src/client/responses.rs`
- [x] Create `src/client/cache.rs`
- [x] Update `src/lib.rs` to export new types
- [x] Update `src/errors.rs` with `ClientApiError` variant
- [x] Update `src/server/api_error.rs` to handle `ClientApiError`

### 8.8 Tests ✅

- [x] Unit tests for `ClientApiError` parsing (5 tests)
- [x] Unit tests for `validate_offline()` grace period logic (3 tests)
- [x] Unit tests for cache encryption/decryption (2 tests)
- [x] Unit tests for tamper detection (modified cache should fail)
- [x] Legacy integration tests updated for new License struct
- [x] Integration tests for bind → validate → heartbeat → release flow (`integration_test_v1_api_lifecycle`)
- [x] Unit tests for offline validation with valid grace period
- [x] Unit tests for offline validation with expired grace period

### 8.9 Documentation ✅

- [x] Update doc comments in `src/client/license.rs` with usage examples
- [x] Update README.md with new client API examples
- [x] Update CHANGELOG.md with Phase 8 changes
- [x] Add code examples for air-gapped system usage (in README.md)

---

## Phase 9: Public Documentation & Examples (P0 - Critical)

Comprehensive, production-ready documentation for users integrating Talos into their projects.

### 9.1 Documentation Structure (`docs/`) ✅

- [x] Create `docs/guide/` directory for user guides
- [x] Create `docs/examples/` directory for complete code examples
- [x] Create `docs/api/` directory for API reference

### 9.2 Getting Started Guide (`docs/guide/getting-started.md`) ✅

- [x] Installation instructions (Cargo.toml setup)
- [x] Feature flag selection guide (which features to enable for your use case)
- [x] Minimal working example (5-minute quickstart)
- [x] Environment setup (config.toml, .env, database)
- [x] Running the server locally
- [x] First license creation and validation

### 9.3 Client Integration Guide (`docs/guide/client-integration.md`) ✅

- [x] Adding Talos to your application
- [x] License struct overview and lifecycle
- [x] Hardware binding explanation (how fingerprinting works)
- [x] **Binding a license** - Complete example with error handling
- [x] **Validating a license** - Online validation flow
- [x] **Offline validation** - Air-gapped system support with grace periods
- [x] **Feature gating** - Checking feature access in your app
- [x] **Heartbeat integration** - Background heartbeat setup
- [x] **Releasing a license** - Proper cleanup on app exit
- [x] Error handling patterns (matching on `ClientErrorCode`)
- [x] Retry strategies for network failures

### 9.4 Server Deployment Guide (`docs/guide/server-deployment.md`) ✅

- [x] Database setup (SQLite vs PostgreSQL)
- [x] Running migrations
- [x] Configuration reference (all config.toml options)
- [x] Environment variables reference
- [x] Production checklist (security, performance, monitoring)
- [x] Docker deployment
- [x] Reverse proxy setup (nginx, traefik)
- [x] TLS/HTTPS configuration
- [x] Health monitoring and alerting

### 9.5 Admin API Guide (`docs/guide/admin-api.md`) ✅

- [x] Enabling the admin-api feature
- [x] Authentication setup (JWT tokens)
- [x] **Creating licenses** - Single and batch creation
- [x] **Managing licenses** - Update, suspend, revoke, reinstate
- [x] **Organization management** - Grouping licenses by org
- [x] **Feature and tier management** - Configuring tiers
- [x] **Monitoring** - Listing licenses, checking status
- [x] **Security** - Protecting admin endpoints
- [x] API token management (creating service accounts)

### 9.6 Advanced Topics Guide _(Deferred)_

- [ ] Custom hardware fingerprinting
- [ ] Extending the tier system
- [ ] Webhook integration (future)
- [ ] Background jobs configuration
- [ ] Rate limiting tuning
- [ ] Database optimization
- [ ] High availability setup
- [ ] Backup and recovery

### 9.7 API Reference (`docs/api/`) ✅

- [x] REST API reference (all endpoints with request/response examples)
- [x] Error codes reference (all error codes with descriptions)
- [x] OpenAPI specification (`docs/api/openapi.json`)
- [x] Interactive Swagger UI documentation
- [ ] _(Deferred)_ Generate rustdoc with `cargo doc --all-features`
- [ ] _(Deferred)_ Host rustdoc on GitHub Pages or docs.rs
- [ ] _(Deferred)_ Configuration reference (all config options with defaults)

### 9.8 Complete Code Examples (`docs/examples/`) ✅

- [x] `examples/basic-client/` - Minimal client integration with runtime license key entry
- [x] `examples/air-gapped/` - Offline validation with grace period handling and `--offline` flag
- [x] `examples/feature-gating/` - Enabling/disabling features based on license tier

**Completed Examples Include:**
- Full README with step-by-step instructions
- PowerShell (Windows) and bash (Mac/Linux) command variants
- Runtime license key entry (env var, file, or user prompt)
- Error handling and validation
- Feature checking patterns

**Deferred Examples:**
- [ ] `examples/desktop-app/` - Desktop application with license dialog
- [ ] `examples/cli-tool/` - CLI tool with license validation
- [ ] `examples/web-service/` - Web service checking licenses per-request
- [ ] `examples/admin-dashboard/` - Simple admin UI for license management
- [ ] `examples/docker-compose/` - Complete Docker deployment with PostgreSQL

### 9.9 Migration Guides _(Deferred)_

- [ ] `docs/guide/migration-v1.md` - Migrating from legacy API to v1 API
- [ ] `docs/guide/migration-activate-to-bind.md` - activate() → bind() migration

### 9.10 Troubleshooting Guide (`docs/guide/troubleshooting.md`) ✅

- [x] Common errors and solutions
- [x] Debugging license validation failures
- [x] Hardware ID changes (why validation might fail)
- [x] Network connectivity issues
- [x] Database connection problems
- [x] FAQ section

---

### Phase 9 Summary

**Completed:**
- 5 comprehensive user guides (getting-started, client-integration, server-deployment, admin-api, troubleshooting)
- REST API reference with all endpoints documented
- OpenAPI 3.1.0 specification with Swagger UI
- 3 working code examples with cross-platform instructions

**Deferred:**
- Advanced topics guide
- Additional code examples (desktop-app, cli-tool, web-service, admin-dashboard, docker-compose)
- Migration guides
- rustdoc generation and hosting

---

## Phase 10: Testing ✅

**Current test count: 220 tests** (135 unit + 39 admin API + 46 integration/other)

### 10.1 Unit Tests ✅

- [x] License key generation uniqueness (1000+ keys) - `license_key.rs`
- [x] License key format validation - `license_key.rs` (12 tests)
- [x] JWT token creation and validation - `auth.rs` (14 tests)
- [x] Tier configuration lookups - `tiers.rs` (4 tests)
- [x] Feature permission logic - `client_api.rs`
- [x] Quota calculation logic - `admin.rs`
- [x] Error code mapping - `api_error.rs` (4 tests)
- [x] Request validation - `validation.rs` (12 tests)
- [x] Encryption/decryption - `encryption.rs` (3 tests)
- [x] Cache security - `cache.rs` (7 tests)
- [x] Token management - `tokens.rs` (8 tests)
- [x] Rate limiting - `rate_limit.rs` (6 tests)
- [x] IP whitelist - `ip_whitelist.rs` (17 tests)

### 10.2 Integration Tests ✅

- [x] Full license lifecycle (create -> bind -> validate -> release) - `integration_tests.rs`
- [x] Bind/release workflow - `integration_tests.rs`
- [x] Feature validation - `integration_tests.rs`
- [x] Heartbeat flow - `integration_tests.rs`
- [x] Admin API CRUD operations - `admin_api_tests.rs` (39 tests)
- [x] Background job execution - `jobs_tests.rs` (7 tests)
- [x] Blacklist behavior - `admin_api_tests.rs`
- [x] Grace period flow - `admin_api_tests.rs`
- [ ] _(Deferred)_ Multi-license organization flow
- [ ] _(Deferred)_ Quota exceeded flow
- [ ] _(Deferred)_ Tier upgrade/downgrade

### 10.3 Load Tests _(Deferred)_

- [ ] Validation endpoint: target 1000 req/s
- [ ] Heartbeat endpoint: target 500 req/s
- [ ] Document performance baselines
- [ ] Identify and address bottlenecks

---

## Phase 11: Deployment & Operations (P1 - High) ✅

### 11.1 Configuration ✅

- [x] Document all environment variables (`.env.example`)
- [x] Create example `.env` file with JWT/rate-limiting vars
- [x] Create example `config.toml` for production deployment (all sections documented)
- [x] Add configuration validation on startup (`config.rs` validates on init)

### 11.2 Docker ✅

- [x] Create optimized Dockerfile (multi-stage build, non-root user)
- [x] Create docker-compose.yml with PostgreSQL
- [x] Document container deployment (in docker-compose.yml comments)
- [x] Add health check to container (`/health` endpoint)

### 11.3 Database Migrations ✅

- [x] Document migration process (comments in SQL files)
- [x] Create SQLite migration script (`migrations/init_sqlite.sql`)
- [x] Create PostgreSQL migration script (`migrations/init_postgres.sql`)
- [ ] _(Deferred)_ Test migration from legacy schema to current schema

---

## Phase 12: Security Enhancements (P1 - High)

### 12.1 Admin API IP Whitelisting ✅

**Critical security feature** - Restrict Admin API access to specific IP addresses.

- [x] Add `admin_ip_whitelist` configuration option (`config.rs`, `AdminConfig`)
- [x] Create IP whitelist middleware for admin routes (`ip_whitelist.rs`)
- [x] Support CIDR notation (e.g., `10.0.0.0/8`, `192.168.1.0/24`)
- [x] Support individual IPs and ranges (IPv4 and IPv6)
- [x] Return 403 Forbidden for non-whitelisted IPs
- [x] Log blocked access attempts (with tracing)
- [x] Document configuration in `config.toml.example`

**Configuration example:**
```toml
[admin]
ip_whitelist = ["127.0.0.1", "10.0.0.0/8", "192.168.0.0/16"]
```

**Implementation details:**
- `IpNetwork` enum handles single IPs and CIDR ranges
- `IpWhitelist` struct with `is_allowed()` method
- `IpWhitelistLayer` and `IpWhitelistMiddleware` for axum integration
- Checks `X-Forwarded-For` and `X-Real-IP` headers for proxy support
- Empty whitelist = disabled (all IPs allowed)
- 17 unit tests for CIDR parsing and matching

### 12.2 Audit Logging _(Deferred)_

- [ ] Log all admin API actions with user/token ID
- [ ] Log license state changes (create, revoke, suspend, etc.)
- [ ] Log authentication failures
- [ ] Configurable audit log destination (file, database, external service)

### 12.3 API Key Rotation _(Deferred)_

- [ ] Add key rotation endpoint for tokens
- [ ] Support overlapping validity periods during rotation
- [ ] Add key rotation reminders/warnings

---

## Phase 13: Encryption API Hardening

_Type-system improvements for compile-time safety. Ergonomic fixes, not security fixes._

### 13.1 Fixed-Size Key Types

- [ ] Change `encrypt_bytes(msg: &[u8], key: &[u8])` to use `key: &[u8; 32]`
- [ ] Change `decrypt_bytes` signature similarly
- [ ] Update all call sites to pass fixed-size arrays

### 13.2 Newtype Wrappers

- [ ] Create `EncryptionKey([u8; 32])` newtype to prevent argument swapping
- [ ] Create `Plaintext(Vec<u8>)` wrapper (optional)
- [ ] Update encrypt/decrypt signatures to use newtypes

### 13.3 Explicit Ciphertext Structure

- [ ] Create `Ciphertext { nonce: [u8; 12], payload: Vec<u8> }` struct
- [ ] Replace `Vec<u8>` return with `Ciphertext` in `encrypt_bytes`
- [ ] Update `decrypt_bytes` to accept `&Ciphertext`
- [ ] Add serialization methods for wire format

### 13.4 Separate Wire Format Layer

- [ ] Move base64 encoding out of encryption module
- [ ] Create dedicated `wire` or `serialization` module
- [ ] Clear separation: encryption returns `Ciphertext`, serialization handles encoding

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
Phase 2 (Device Mgmt + Rate Limiting) ────────────────────┐
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
Phase 6 (Blacklist & Validation) ◄────────────────────────┘
         │
         ▼
Phase 7 (OpenAPI & Logging)
         │
         ▼
Phase 8 (Client Updates)
         │
         ▼
Phase 9 (Public Docs & Examples)
         │
         ▼
Phase 10 (Testing) ────► Phase 11 (Deployment)
```

---

## Quick Reference: New Endpoints

### Admin Endpoints (JWT Required)

| Method | Endpoint                                  | Description              |
| ------ | ----------------------------------------- | ------------------------ |
| POST   | `/api/v1/licenses`                        | Create single license    |
| POST   | `/api/v1/licenses/batch`                  | Create multiple licenses |
| GET    | `/api/v1/licenses?org_id={id}`            | List org's licenses      |
| GET    | `/api/v1/licenses/{license_id}`           | Get license details      |
| PATCH  | `/api/v1/licenses/{license_id}`           | Update tier/features     |
| POST   | `/api/v1/licenses/{license_id}/revoke`    | Suspend/revoke           |
| POST   | `/api/v1/licenses/{license_id}/reinstate` | Reinstate                |
| POST   | `/api/v1/licenses/{license_id}/extend`    | Extend expiry            |
| POST   | `/api/v1/licenses/{license_id}/release`   | Admin force unbind       |
| POST   | `/api/v1/licenses/{license_id}/blacklist` | Permanent ban            |
| PATCH  | `/api/v1/licenses/{license_id}/usage`     | Update bandwidth         |
| POST   | `/api/v1/tokens`                          | Create API token         |
| GET    | `/api/v1/tokens`                          | List all tokens          |
| GET    | `/api/v1/tokens/{token_id}`               | Get token details        |
| DELETE | `/api/v1/tokens/{token_id}`               | Revoke token             |

### Client Endpoints (Rate-Limited, No Auth)

| Method | Endpoint                          | Description              |
| ------ | --------------------------------- | ------------------------ |
| POST   | `/api/v1/client/bind`             | Bind license to hardware |
| POST   | `/api/v1/client/release`          | Release from hardware    |
| POST   | `/api/v1/client/validate`         | Validate license         |
| POST   | `/api/v1/client/validate-or-bind` | Validate or auto-bind    |
| POST   | `/api/v1/client/validate-feature` | Check feature access     |
| POST   | `/api/v1/client/heartbeat`        | Liveness ping            |

---

## Estimated Effort by Phase

| Phase | Description          | Complexity |
| ----- | -------------------- | ---------- |
| 1     | Core Admin API       | High       |
| 2     | Device Management    | Medium     |
| 3     | Feature Gating       | Low        |
| 4     | Lifecycle Management | Medium     |
| 5     | Background Jobs      | Medium     |
| 6     | Security             | Medium     |
| 7     | Documentation        | Low        |
| 8     | Client Updates       | Medium     |
| 9     | Testing              | High       |
| 10    | Deployment           | Low        |

---

## Notes

### Open Source Design Principles

- **No Product-Specific Code**: The Talos codebase should never contain product-specific naming, constants, or logic. Keep the library generic.
- **Configuration Over Code**: All customization (key prefix, tiers, features) happens via configuration, not code changes.
- **Feature Flags**: Advanced features are opt-in via Cargo features. A simple use case shouldn't require complex setup.
- **Sensible Defaults**: Works out of the box with SQLite and basic endpoints. Advanced features require explicit opt-in.

### Migration & Compatibility

- **Breaking Changes**: The schema changes in Phase 1.1 are breaking. Existing Talos deployments will need migration.
- **Backwards Compatibility**: Old `/activate`, `/validate`, `/deactivate`, `/heartbeat` endpoints can be deprecated but kept for transition period.
- **Testing Strategy**: Write tests alongside implementation, not after. Each task should include its tests.

### Example Configuration

Here's an example of how a production deployment might configure Talos:

```toml
[license]
key_prefix = "MYAPP"

[auth]
enabled = true
jwt_secret = "env:TALOS_JWT_SECRET"

[quota]
enabled = true

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

This is **configuration**, not code. Each deployment configures its own tiers and key prefix.
