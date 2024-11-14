use axum::{Router, routing::post};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use talos::server::database::Database;
use talos::server::handlers::{activate_license_handler, validate_license_handler, deactivate_license_handler, heartbeat_handler}; // Import the heartbeat handler
use tracing_subscriber;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Initialize the database (Database::new already returns an Arc<Database>)
    let db = Database::new().await; // Do not wrap this in Arc::new

    // Set up the Axum router with routes for license management
    let app = Router::new()
        .route("/activate", post(activate_license_handler))
        .route("/validate", post(validate_license_handler))
        .route("/deactivate", post(deactivate_license_handler))
        .route("/heartbeat", post(heartbeat_handler))  // Add the heartbeat route
        .with_state(db); // Use the Arc<Database> directly

    // Bind to an address using TcpListener
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");
    println!("Server listening on http://{}", addr);

    // Serve the application using axum::serve
    axum::serve(listener, app.into_make_service())
        .await
        .expect("Server failed to start");
}
