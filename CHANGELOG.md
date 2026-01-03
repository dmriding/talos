# Changelog
All notable changes to **Talos** will be documented in this file.

This project uses **Calendar Versioning (CalVer)**
Format: `vYYYY.MM.INCREMENT`

---

## v2025.12.2 — 2025-12-XX

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

### Changed
- Updated `BindingAction` enum with `SystemRelease` variant for background job auditing
- Added `release_license` database method for clearing hardware bindings
- License struct now tracks `grace_period_ends_at`, `suspended_at`, `revoked_at`, `revoke_reason`, `suspension_message`

### Tests
- 19 new integration tests for Phase 4 lifecycle endpoints
- 7 new integration tests for Phase 5 background jobs
- Total test count: 130+ tests passing

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
