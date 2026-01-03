use std::net::SocketAddr;

use tokio::net::TcpListener;
use tracing::{info, warn};

use talos::config::init_config;
use talos::errors::{LicenseError, LicenseResult};
use talos::server::bootstrap::{check_bootstrap_token, execute_token_command, parse_token_command};
use talos::server::database::Database;
use talos::server::handlers::AppState;
use talos::server::routes::build_router;

#[cfg(feature = "jwt-auth")]
use talos::server::auth::AuthState;

#[tokio::main]
async fn main() -> LicenseResult<()> {
    // Parse CLI arguments for token commands
    let args: Vec<String> = std::env::args().collect();
    let token_cmd = parse_token_command(&args);

    // Load and validate configuration first
    let config = init_config()?;

    // Initialize logging based on config
    if config.logging.enabled {
        tracing_subscriber::fmt::init();
    }

    // Initialize the database from config
    let db = Database::new().await?;

    // Check for CLI token commands (these run and exit)
    if execute_token_command(&db, token_cmd).await? {
        return Ok(()); // Command executed, exit
    }

    // Check for bootstrap token on startup
    if let Some(raw_token) = check_bootstrap_token(&db).await? {
        warn!("═══════════════════════════════════════════════════════");
        warn!("BOOTSTRAP TOKEN CREATED - SAVE THIS VALUE:");
        warn!("{}", raw_token);
        warn!("═══════════════════════════════════════════════════════");
    }

    // Build auth state if jwt-auth feature is enabled
    #[cfg(feature = "jwt-auth")]
    let auth = AuthState::from_config(&config.auth)?;

    // Build shared app state
    let state = AppState {
        db,
        #[cfg(feature = "jwt-auth")]
        auth,
    };

    // Set up the Axum router with all routes (legacy, client API, admin API, Swagger UI)
    let app = build_router(state);

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
