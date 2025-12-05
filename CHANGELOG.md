# Changelog
All notable changes to **Talos** will be documented in this file.

This project uses **Calendar Versioning (CalVer)**  
Format: `vYYYY.MM.INCREMENT`

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
