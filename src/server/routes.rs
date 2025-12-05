use axum::{routing::post, Router};

use crate::server::handlers::{
    activate_license_handler,
    deactivate_license_handler,
    heartbeat_handler,
    validate_license_handler,
    AppState,
};

/// Build the main application router for the Talos server.
///
/// This is a convenience helper so `main.rs` or tests can
/// construct the router in a single call.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/activate", post(activate_license_handler))
        .route("/validate", post(validate_license_handler))
        .route("/deactivate", post(deactivate_license_handler))
        .route("/heartbeat", post(heartbeat_handler))
        .with_state(state)
}
