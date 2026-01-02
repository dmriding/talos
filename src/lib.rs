//! Talos - A secure licensing system for Rust applications
//!
//! # Features
//!
//! Talos uses feature flags to allow you to include only what you need:
//!
//! - `server` - Server components (handlers, database). Enabled by default.
//! - `sqlite` - SQLite database backend. Enabled by default.
//! - `postgres` - PostgreSQL database backend.
//!
//! # Example
//!
//! ```toml
//! # Use defaults (server + sqlite)
//! talos = { git = "https://github.com/dmriding/talos" }
//!
//! # Client-only (no server components)
//! talos = { git = "https://github.com/dmriding/talos", default-features = false }
//!
//! # Server with PostgreSQL
//! talos = { git = "https://github.com/dmriding/talos", features = ["server", "postgres"] }
//! ```

// Core modules (always available)
pub mod config;
pub mod encryption;
pub mod errors;
pub mod hardware;
pub mod license_key;
pub mod tiers;

// Client-related modules (always available)
pub mod client {
    pub mod encrypted_storage;
    pub mod heartbeat;
    pub mod key_generation;
    pub mod license;

    // Re-export for backwards compatibility
    pub use license as client;
}

// Server-related modules (requires "server" feature)
#[cfg(feature = "server")]
#[path = "server/mod.rs"]
pub mod server;
