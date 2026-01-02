// src/server/mod.rs

//! Server-side components for Talos.
//!
//! This module contains:
//! - `database`    → DB abstraction over SQLite/Postgres
//! - `handlers`    → Axum HTTP handlers for license endpoints
//! - `routes`      → Router builder (optional helper)
//! - `server_sim`  → In-memory simulator for tests
//! - `auth`        → JWT authentication middleware (requires `jwt-auth` feature)

pub mod database;
pub mod handlers;
pub mod routes;
pub mod server_sim;

#[cfg(feature = "jwt-auth")]
pub mod auth;

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
