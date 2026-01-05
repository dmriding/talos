# Changelog
All notable changes to **Talos** will be documented in this file.

This project uses **Calendar Versioning (CalVer)**
Format: `vYYYY.MM.INCREMENT`

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
