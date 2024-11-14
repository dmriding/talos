// Core modules
pub mod config;
pub mod encryption;
pub mod errors;
pub mod hardware;

// Client-related modules
pub mod client {
    pub mod client;
    pub mod encrypted_storage;
    pub mod heartbeat;
    pub mod key_generation;
}

// Server-related modules
pub mod server {
    pub mod database;
    pub mod handlers;
    pub mod server_sim;
}
