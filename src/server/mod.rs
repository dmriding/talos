// src/server/mod.rs

//! Server-side components for Talos.
//!
//! This module contains:
//! - `database`    → DB abstraction over SQLite/Postgres
//! - `handlers`    → Axum HTTP handlers for license endpoints
//! - `client_api`  → New client API for bind/release/validate
//! - `routes`      → Router builder (optional helper)
//! - `server_sim`  → In-memory simulator for tests
//! - `auth`        → JWT authentication middleware (requires `jwt-auth` feature)
//! - `admin`       → Admin API for license CRUD (requires `admin-api` feature)
//! - `rate_limit`  → Rate limiting middleware (requires `rate-limiting` feature)

pub mod client_api;
pub mod database;
pub mod handlers;
pub mod routes;
pub mod server_sim;

#[cfg(feature = "jwt-auth")]
pub mod auth;

#[cfg(feature = "admin-api")]
pub mod admin;

#[cfg(feature = "rate-limiting")]
pub mod rate_limit;

// Optional: convenient re-exports so callers can do `talos::server::X`
// instead of digging into submodules.

pub use client_api::{
    bind_handler, client_heartbeat_handler, release_handler, validate_feature_handler,
    validate_handler, validate_or_bind_handler, BindRequest, BindResponse, ClientError,
    ClientErrorCode, ClientHeartbeatRequest, ClientHeartbeatResponse, ReleaseRequest,
    ReleaseResponse, ValidateFeatureRequest, ValidateFeatureResponse, ValidateOrBindRequest,
    ValidateRequest, ValidateResponse,
};
pub use database::Database;
pub use handlers::{
    activate_license_handler, deactivate_license_handler, heartbeat_handler,
    validate_license_handler, AppState,
};
pub use routes::build_router;

#[cfg(feature = "jwt-auth")]
pub use auth::{AuthError, AuthState, AuthenticatedUser, Claims, JwtValidator, OptionalUser};

#[cfg(feature = "admin-api")]
pub use admin::{
    admin_release_handler, batch_create_license_handler, create_license_handler,
    get_license_handler, list_licenses_handler, update_license_handler, AdminReleaseRequest,
    AdminReleaseResponse,
};

#[cfg(feature = "rate-limiting")]
pub use rate_limit::{
    create_rate_limiter, rate_limit_error_response, RateLimitType, SmartIpKeyExtractor,
};
