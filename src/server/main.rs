use std::net::SocketAddr;

use axum::{routing::post, Router};
use tokio::net::TcpListener;
use tracing::info;

use talos::config::init_config;
use talos::errors::{LicenseError, LicenseResult};
use talos::server::database::Database;
use talos::server::handlers::{
    activate_license_handler, deactivate_license_handler, heartbeat_handler,
    validate_license_handler, AppState,
};

#[tokio::main]
async fn main() -> LicenseResult<()> {
    // Load and validate configuration first
    let config = init_config()?;

    // Initialize logging based on config
    if config.logging.enabled {
        tracing_subscriber::fmt::init();
    }

    // Initialize the database from config
    let db = Database::new().await?;

    // Build shared app state
    let state = AppState { db };

    // Set up the Axum router with routes for license management
    let app = Router::new()
        .route("/activate", post(activate_license_handler))
        .route("/validate", post(validate_license_handler))
        .route("/deactivate", post(deactivate_license_handler))
        .route("/heartbeat", post(heartbeat_handler))
        .with_state(state);

    // Bind to address from config
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .map_err(|e| LicenseError::ConfigError(format!("invalid server address: {e}")))?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| LicenseError::ServerError(format!("failed to bind to {}: {}", addr, e)))?;

    info!("Talos server listening on http://{}", addr);

    // Serve the application
    axum::serve(listener, app.into_make_service())
        .await
        .map_err(|e| LicenseError::ServerError(format!("server failed: {e}")))?;

    Ok(())
}
