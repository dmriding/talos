use std::net::SocketAddr;

use axum::{routing::post, Router};
use tokio::net::TcpListener;
use tracing::info;

use talos::errors::{LicenseError, LicenseResult};
use talos::server::database::Database;
use talos::server::handlers::{
    activate_license_handler, deactivate_license_handler, heartbeat_handler,
    validate_license_handler, AppState,
};

#[tokio::main]
async fn main() -> LicenseResult<()> {
    // Simple logging init.
    // Respects RUST_LOG, e.g.:
    //   RUST_LOG=info talos_server
    tracing_subscriber::fmt::init();

    // Initialize the database from config.toml
    let db = Database::new().await?; // returns Arc<Database>

    // Build shared app state
    let state = AppState { db };

    // Set up the Axum router with routes for license management
    let app = Router::new()
        .route("/activate", post(activate_license_handler))
        .route("/validate", post(validate_license_handler))
        .route("/deactivate", post(deactivate_license_handler))
        .route("/heartbeat", post(heartbeat_handler))
        .with_state(state);

    // Bind to an address using TcpListener
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| LicenseError::ServerError(format!("failed to bind: {e}")))?;

    info!("Talos server listening on http://{}", addr);

    // Serve the application using axum::serve
    axum::serve(listener, app.into_make_service())
        .await
        .map_err(|e| LicenseError::ServerError(format!("server failed: {e}")))?;

    Ok(())
}
