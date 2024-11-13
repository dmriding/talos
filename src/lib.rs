// Make modules accessible
pub mod config;
pub mod encrypted_storage;
pub mod encryption;
pub mod errors;
pub mod hardware;
pub mod key_generation;

// Client-related modules
pub mod client {
    pub mod client;
    pub mod heartbeat;
}

// Server-related modules
pub mod server {
    pub mod server_sim;
}
