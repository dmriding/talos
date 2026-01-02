// src/server/mod.rs

//! Server-side components for Talos.
//!
//! This module contains:
//! - `database`    → DB abstraction over SQLite/Postgres
//! - `handlers`    → Axum HTTP handlers for license endpoints
//! - `routes`      → Router builder (optional helper)
//! - `server_sim`  → In-memory simulator for tests
//! - `auth`        → JWT authentication middleware (requires `jwt-auth` feature)
//! - `admin`       → Admin API for license CRUD (requires `admin-api` feature)

pub mod database;
pub mod handlers;
pub mod routes;
pub mod server_sim;

#[cfg(feature = "jwt-auth")]
pub mod auth;

#[cfg(feature = "admin-api")]
pub mod admin;

// Optional: convenient re-exports so callers can do `talos::server::X`
// instead of digging into submodules.

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
    batch_create_license_handler, create_license_handler, get_license_handler,
    list_licenses_handler, update_license_handler,
};
