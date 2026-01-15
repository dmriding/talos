# Changelog
All notable changes to **Talos** will be documented in this file.

This project uses **Calendar Versioning (CalVer)**
Format: `vYYYY.MM.INCREMENT`

---

## v0.2.1 — 2026-01-15

### Fixed
- **Fixed CMD window flashing in GUI applications** - WMIC commands for hardware fingerprinting now use `CREATE_NO_WINDOW` flag to prevent console window popups in Tauri/GUI applications on Windows.

---

## v0.2.0 — 2026-01-09

### Fixed

#### Secure License Storage (Bug Fix)
- **Fixed license files stored in project root** - License files (`talos_license.enc`, `talos_cache.enc`) were being stored in the current working directory when talos was used as a dependency. Now properly stored in secure OS locations.

### Added

#### Keyring-Based Storage (`src/client/storage.rs`)
- **OS Keyring Integration** - License data now stored in the OS credential store:
  - Windows: Windows Credential Manager
  - macOS: Keychain
  - Linux: Secret Service (via D-Bus)
- **File Fallback** - Automatic fallback to app data directory if keyring unavailable:
  - Windows: `%APPDATA%\talos\`
  - macOS: `~/Library/Application Support/talos/`
  - Linux: `~/.local/share/talos/`
- **Automatic Migration** - Existing license files in project root are automatically migrated on first load and deleted after successful migration
- **StorageKey Enum** - `StorageKey::License` and `StorageKey::Cache` for type-safe storage operations
- **Public Storage API** - `save_to_storage()`, `load_from_storage()`, `clear_from_storage()`

#### New Dependencies
- `keyring = "3"` - Cross-platform credential storage
- `dirs = "5"` - Platform-specific directory paths

#### Error Handling
- **KeyringError Variant** - New `LicenseError::KeyringError(String)` for keyring-specific failures

### Changed
- `save_license_to_disk()` now uses keyring with file fallback instead of CWD
- `load_license_from_disk()` checks keyring → app data → legacy CWD (with migration)
- `clear_license_from_disk()` clears from all storage locations
- Same changes applied to cache storage functions
- Version bumped to `0.2.0`

### Security
- License data remains AES-256-GCM encrypted with hardware-derived keys
- Keyring adds additional OS-level protection on top of existing encryption
- Hardware binding preserved - credentials cannot be copied between machines

### Migration
- **Fully Automatic** - No user action required
- Existing `talos_license.enc` and `talos_cache.enc` files in project roots will be:
  1. Detected on first `load_from_disk()` call
  2. Migrated to keyring (or app data directory)
  3. Deleted from the project root
- API unchanged - `License::load_from_disk()` and `save_to_disk()` work exactly as before

### Tests
- 5 new unit tests for storage module
- Updated integration tests for new error messages
- Total test count: 214 tests passing

---

## v2025.12.6 — 2026-01-06

### Added

#### Phase 12.1: Admin API IP Whitelisting
- **IP Whitelist Middleware** (`src/server/ip_whitelist.rs`)
  - `IpNetwork` enum for single IPs and CIDR ranges (IPv4/IPv6)
  - `IpWhitelist` struct with `is_allowed()` method
  - `IpWhitelistLayer` and `IpWhitelistMiddleware` for Axum integration
  - Checks `X-Forwarded-For` and `X-Real-IP` headers for proxy support
  - Returns 403 Forbidden with `IP_NOT_ALLOWED` error for blocked IPs
  - Empty whitelist = disabled (all IPs allowed)
- **Admin Config** (`src/config.rs`)
  - `AdminConfig` struct with `ip_whitelist` and `audit_logging` fields
  - `TALOS_ADMIN_IP_WHITELIST` environment variable override
  - Supports comma-separated IPs/CIDRs in env var
- **Route Protection** (`src/server/routes.rs`)
  - IP whitelist layer applied to all admin routes
  - Works with both `admin-api` only and `admin-api` + `jwt-auth` configurations

### Changed
- Server now uses `into_make_service_with_connect_info::<SocketAddr>()` for IP extraction
- Added `#[derive(Default)]` to `AdminConfig` and `IpWhitelist` (clippy fix)
- Simplified async blocks in middleware (clippy fix)

### Configuration

```toml
[admin]
# Restrict admin API to specific IPs/networks
ip_whitelist = ["127.0.0.1", "::1", "10.0.0.0/8"]
audit_logging = false
```

### Tests
- 17 new unit tests for IP whitelist parsing and matching
- Updated admin API tests to disable IP whitelist via env var
- Total test count: 220 tests passing (135 unit + 39 admin API + 46 integration)

### Documentation
- Updated `docs/guide/admin-api.md` with built-in IP whitelist section
- Updated `docs/guide/server-deployment.md` production checklist
- Updated `config.toml.example` with `[admin]` section
- Updated ROADMAP.md Phase 10 test count and Phase 12.1 completion

---

## v2025.12.5 — 2026-01-05

### Added

#### Phase 9: Public Documentation & Examples
- **Complete Code Examples** (`docs/examples/`)
  - `basic-client/` - Minimal client integration with runtime license key entry
  - `air-gapped/` - Offline validation with encrypted cache, grace periods, and `--offline` flag
  - `feature-gating/` - Enable/disable features based on license tier with upgrade prompts
- **Comprehensive User Guides** (`docs/guide/`)
  - `getting-started.md` - 5-minute quickstart with feature flags and setup
  - `client-integration.md` - Full client lifecycle, offline validation, feature gating
  - `server-deployment.md` - Database setup, Docker, nginx/traefik, production checklist
  - `admin-api.md` - All admin endpoints, authentication, security best practices
  - `troubleshooting.md` - Common errors, debugging, FAQ
- **Cross-Platform Documentation**
  - All examples include PowerShell (Windows) and bash (Mac/Linux) instructions
  - Environment variable syntax for both platforms (`$env:VAR` vs `VAR=value`)
  - File operations for both platforms (`Remove-Item` vs `rm`)

### Fixed
- **Date Format Bug in Offline Validation** (`src/server/client_api.rs`, `src/server/admin.rs`)
  - Server was returning dates as `NaiveDateTime.to_string()` (space-separated format)
  - Changed to `.and_utc().to_rfc3339()` for proper RFC3339 format parsing in cache
  - Fixes offline validation always failing with "grace period expired"

### Documentation
- Updated README.md with examples section and Phase 9 status
- All example READMEs include step-by-step instructions
- PowerShell and bash command variants throughout

---

## v2025.12.4 — 2026-01-05

### Added

#### Phase 8: Client Library Updates
- **New Client Error Types** (`src/client/errors.rs`)
  - `ClientErrorCode` enum matching server error codes (LICENSE_NOT_FOUND, LICENSE_EXPIRED, etc.)
  - `ClientApiError` struct for typed API error handling
  - `ClientApiError::from_response()` for async error parsing
  - `From<reqwest::Error>` conversion for network errors
- **Response Types** (`src/client/responses.rs`)
  - `ValidationResult` - Online validation response (features, tier, expires_at, warning)
  - `BindResult` - Hardware binding response (license_id, features, tier, expires_at)
  - `FeatureResult` - Feature validation response (allowed, message, tier)
  - `HeartbeatResult` - Heartbeat response (server_time, grace_period_ends_at)
  - Helper methods: `ValidationResult::has_feature()`, `BindResult::has_feature()`
- **Secure Cache for Offline Validation** (`src/client/cache.rs`)
  - `CachedValidation` struct for encrypted offline validation data
  - AES-256-GCM encryption with hardware-bound key derivation
  - Salt-based key derivation using SHA-256 HKDF
  - Helper methods: `is_valid_for_offline()`, `is_license_expired()`, `grace_period_remaining()`, `matches_hardware()`, `has_feature()`
  - Tamper detection via GCM authentication tag
- **New License Methods** (`src/client/license.rs`)
  - `License::new(license_key, server_url)` - Primary constructor
  - `bind(device_name, device_info) -> BindResult` - Hardware binding (replaces `activate()`)
  - `release()` - Release hardware binding (replaces `deactivate()`)
  - `validate() -> ValidationResult` - Online validation with cache update
  - `validate_offline() -> ValidationResult` - Cached validation for air-gapped systems
  - `validate_with_fallback() -> ValidationResult` - Online with offline fallback
  - `validate_feature(feature) -> FeatureResult` - Server-side feature check
  - `heartbeat() -> HeartbeatResult` - Heartbeat with grace period update
- **Air-Gapped System Support**
  - Server-provided grace periods stored in encrypted cache
  - `validate_offline()` returns warning when grace period nearing expiration
  - Cache automatically updated on each `validate()` or `heartbeat()` call
  - Hardware binding ensures cache cannot be copied between machines

### Changed
- `License` struct now uses `license_key` as primary identifier (not `license_id`)
- `License::bind()` now sets `hardware_id` automatically from system fingerprint
- `validate()` updates the encrypted cache on success
- `heartbeat()` updates the cache grace period on success
- Legacy `activate()` and `deactivate()` marked as `#[deprecated]` with migration guidance
- `LicenseError` enum extended with `ClientApiError(ClientApiError)` variant

### Deprecated
- `License::activate()` - Use `License::bind()` instead
- `License::deactivate()` - Use `License::release()` instead
- Legacy fields (`license_id`, `client_id`, `expiry_date`, `signature`, `is_active`) - Use new v1 API fields

### Tests
- 5 new unit tests for `ClientApiError` parsing
- 4 new unit tests for response type parsing
- 8 new unit tests for cache encryption/decryption and tamper detection
- 3 new unit tests for offline validation grace period logic
- Integration test `integration_test_v1_api_lifecycle` for full v1 API flow
- Updated existing tests for new `License::new()` constructor
- Total test count: 192 tests passing

### Documentation
- Updated README.md with new client API examples
- Added air-gapped system usage examples
- Added code examples for `validate_with_fallback()`

---

## v2025.12.3 — 2026-01-05

### Added

#### Phase 7.1: OpenAPI Specification
- `openapi` feature flag for optional OpenAPI/Swagger integration
- `utoipa` and `utoipa-swagger-ui` dependencies for OpenAPI 3.0 generation
- `/swagger-ui` endpoint for interactive API documentation
- `/api-docs/openapi.json` endpoint for OpenAPI specification
- `#[utoipa::path]` annotations on all handler functions
- `ToSchema` derives on all request/response structs
- `ApiDoc` and `ApiDocWithAdmin` structs for conditional endpoint documentation
- Bearer token authentication scheme documented in OpenAPI spec

#### Phase 7.3: Logging & Observability
- **Request Logging Middleware** (`src/server/logging.rs`)
  - Automatic request ID generation (UUID v4)
  - `X-Request-Id` header added to all responses
  - Request timing with millisecond precision
  - Tracing spans for request context
- **Health Check Endpoint** (`GET /health`)
  - Service status ("healthy" or "degraded")
  - Database connectivity check
  - Database type reporting (sqlite/postgres)
  - Service version from `Cargo.toml`
- **Structured License Event Logging**
  - `LicenseEvent` enum for all license state changes
  - `log_license_event()` for state changes (created, revoked, etc.)
  - `log_license_binding_event()` for hardware binding events
  - Events: `Created`, `Bound`, `Released`, `Validated`, `ValidationFailed`, `Activated`, `Deactivated`, `Revoked`, `Reinstated`, `Suspended`, `Extended`, `Blacklisted`, `Heartbeat`, `UsageUpdated`
- Health endpoint added to OpenAPI documentation with "system" tag

#### Phase 7.2: Error Response Standardization
- **New `ApiError` struct** - Unified error response format across all endpoints:
  ```json
  {
    "error": {
      "code": "LICENSE_NOT_FOUND",
      "message": "The requested license does not exist",
      "details": null
    }
  }
  ```
- **`ErrorCode` enum** - 27 machine-readable error codes:
  - License state: `LICENSE_NOT_FOUND`, `LICENSE_EXPIRED`, `LICENSE_REVOKED`, `LICENSE_SUSPENDED`, `LICENSE_BLACKLISTED`, `LICENSE_INACTIVE`
  - Hardware binding: `ALREADY_BOUND`, `NOT_BOUND`, `HARDWARE_MISMATCH`
  - Features/quotas: `FEATURE_NOT_INCLUDED`, `QUOTA_EXCEEDED`
  - Validation: `INVALID_REQUEST`, `MISSING_FIELD`, `INVALID_FIELD`
  - Authentication: `MISSING_TOKEN`, `INVALID_HEADER`, `INVALID_TOKEN`, `TOKEN_EXPIRED`, `INSUFFICIENT_SCOPE`, `AUTH_DISABLED`
  - Server errors: `NOT_FOUND`, `CONFLICT`, `DATABASE_ERROR`, `CONFIG_ERROR`, `CRYPTO_ERROR`, `NETWORK_ERROR`, `INTERNAL_ERROR`
- **Convenience constructors** - `ApiError::license_not_found()`, `invalid_field()`, `missing_field()`, `not_found()`, `database_error()`, `internal_error()`
- **Error conversions** - `From<LicenseError>`, `From<AdminError>`, `From<ClientError>`, `From<AuthError>` for `ApiError`
- Error codes documented in README.md with HTTP status mappings

### Changed
- `main.rs` now uses `build_router()` instead of manually creating routes (enables Swagger UI)
- `LicenseError::IntoResponse` now delegates to `ApiError` for consistent format
- `AdminError::IntoResponse` now delegates to `ApiError`
- `ClientError::IntoResponse` now delegates to `ApiError`
- `AuthError::IntoResponse` now delegates to `ApiError`
- Updated test assertions to use new error response format (`body["error"]["message"]`)

### Fixed
- Duplicate `AuthState` import in `main.rs` from merge conflict
- Swagger UI returning 404 (main.rs wasn't using `build_router()`)

### Tests
- 4 new unit tests for `ApiError` and `ErrorCode`
- 3 new unit tests for `LicenseEvent` and health response
- Updated 16 admin API tests for new error format
- Total test count: 168+ tests passing

---

## v2025.12.2 — 2026-01-03

### Added

#### Phase 3: Feature Gating
- `rate-limiting` feature flag with Tower governor middleware integration
- `/api/v1/licenses/{id}/validate-feature` endpoint for tier-based feature access control
- `ValidateFeatureRequest` and `ValidateFeatureResponse` types

#### Phase 4: License Lifecycle Management
- **Suspend License** (`/api/v1/licenses/{id}/suspend`)
  - Configurable grace period (default 7 days)
  - Automatic revocation after grace period expires
  - Optional suspension message
- **Revoke License** (`/api/v1/licenses/{id}/revoke`)
  - Permanent license termination with reason tracking
  - Clears hardware binding on revocation
- **Reinstate License** (`/api/v1/licenses/{id}/reinstate`)
  - Restore suspended or revoked licenses to active status
  - Optional new expiration date and bandwidth reset
- **Extend License** (`/api/v1/licenses/{id}/extend`)
  - Update license expiration date
  - Optional bandwidth reset
- **Update Usage** (`/api/v1/licenses/{id}/usage`)
  - Track bandwidth usage and limits
  - Calculate quota exceeded status and usage percentage

#### Phase 5: Background Jobs
- `background-jobs` feature flag with `tokio-cron-scheduler` integration
- `JobScheduler` struct for managing scheduled tasks
- `JobConfig` for configurable cron schedules
- **Grace Period Expiration Job**: Automatically revokes suspended licenses past their grace period
- **License Expiration Job**: Marks expired licenses as 'expired' status
- **Stale Device Cleanup Job**: Releases licenses from devices not seen for configurable days (default 90)
- Manual trigger methods for all jobs (`run_*_now`)
- Database methods: `get_expired_grace_period_licenses`, `get_expired_licenses`, `get_stale_device_licenses`
- `SystemRelease` binding action for audit trail

#### Phase 6: Blacklist & Security
- **Blacklist License** (`/api/v1/licenses/{id}/blacklist`)
  - Permanently ban licenses for abuse, fraud, or policy violations
  - Sets `is_blacklisted = true` and status to 'revoked'
  - Clears hardware binding on blacklist
  - Prevents reinstatement through normal reinstate endpoint
  - Requires reason for audit trail
- **Request Validation Module** (`src/server/validation.rs`)
  - `validate_uuid()` - Validate UUID format (xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)
  - `validate_license_key()` - Validate license key format (PREFIX-XXXX-XXXX-XXXX)
  - `validate_hardware_id()` - Validate SHA-256 hardware fingerprint (64 hex chars)
  - `validate_datetime()` - Validate ISO 8601 datetime formats
  - `validate_feature_name()` - Validate feature name format
  - `validate_org_id()` - Validate organization ID format
  - `validate_not_empty()`, `validate_length()` - String validation utilities
- Added `regex` dependency for validation patterns

### Changed
- Updated `BindingAction` enum with `SystemRelease` variant for background job auditing
- Added `release_license` database method for clearing hardware bindings
- License struct now tracks `grace_period_ends_at`, `suspended_at`, `revoked_at`, `revoke_reason`, `suspension_message`
- Reinstate endpoint now blocks blacklisted licenses

### Tests
- 19 new integration tests for Phase 4 lifecycle endpoints
- 7 new integration tests for Phase 5 background jobs
- 6 new integration tests for Phase 6 blacklist endpoint
- 12 new unit tests for validation module
- 5 new doctests for validation functions
- Total test count: 150+ tests passing

---

## v2025.12.1 — 2025-12-05
### Added
- Full client ↔ server license lifecycle:
  - Activation, validation, deactivation, heartbeat.
  - Client automatically binds licenses to hashed hardware fingerprint.
- AES-GCM encrypted local storage for client-side secrets.
- Secure key generation module combining hardware ID, timestamp, and encrypted private key.
- Complete end-to-end test suite:
  - Unit tests for encryption, config, and storage.
  - Server tests for each endpoint.
  - Heartbeat tests using in-memory SQLite.
  - Integration tests verifying the entire license lifecycle.
- Example programs (`manual_activate.rs`, `talos_client`, `talos_server`) for manual and automated verification.
- Re-exported router, handlers, and database types for cleaner server usage.

### Changed
- Refactored project directory layout for clear separation between:
  - Client
  - Server
  - Encryption
  - Hardware
  - Config
  - Errors
- Improved error propagation using unified `LicenseError` + `LicenseResult`.
- Updated `.gitignore` to exclude:
  - Local SQLite databases (`talos_dev.db`)
  - Encrypted storage file (`talos_encrypted_data`)
  - Environment files

### Fixed
- Integration tests failing due to missing router exports — resolved with `build_router` re-export.
- Client attempting to hit wrong server URLs — corrected with `get_server_url()` precedence logic.
- Heartbeat handler not updating `last_heartbeat` — now persists timestamp correctly.
- AES-GCM decryption errors from invalid keys — now validated and fully tested.
- In-memory database tests failing due to missing schema — schema now created automatically.

---

## v2025.12.0 — Initial 2025 Release
### Added
- Initial module scaffolding.
- Basic structure for client, server, encryption, and hardware fingerprinting.
- Early Axum + SQLx integration.
