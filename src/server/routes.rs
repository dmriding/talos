use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::server::handlers::{
    activate_license_handler, deactivate_license_handler, heartbeat_handler,
    validate_license_handler, AppState,
};

#[cfg(feature = "admin-api")]
use crate::server::admin::{
    batch_create_license_handler, create_license_handler, get_license_handler,
    list_licenses_handler, update_license_handler,
};

/// Build the main application router for the Talos server.
///
/// This is a convenience helper so `main.rs` or tests can
/// construct the router in a single call.
///
/// # Routes
///
/// ## Client endpoints (always available)
/// - `POST /activate` - Activate a license
/// - `POST /validate` - Validate a license
/// - `POST /deactivate` - Deactivate a license
/// - `POST /heartbeat` - Send heartbeat
///
/// ## Admin endpoints (requires `admin-api` feature)
/// - `POST /api/v1/licenses` - Create a license
/// - `POST /api/v1/licenses/batch` - Batch create licenses
/// - `GET /api/v1/licenses/{license_id}` - Get a license
/// - `GET /api/v1/licenses` - List licenses (requires org_id query param)
/// - `PATCH /api/v1/licenses/{license_id}` - Update a license
pub fn build_router(state: AppState) -> Router {
    let router = Router::new()
        // Client endpoints
        .route("/activate", post(activate_license_handler))
        .route("/validate", post(validate_license_handler))
        .route("/deactivate", post(deactivate_license_handler))
        .route("/heartbeat", post(heartbeat_handler));

    // Add admin API routes if feature is enabled
    #[cfg(feature = "admin-api")]
    let router = router
        .route("/api/v1/licenses", post(create_license_handler))
        .route("/api/v1/licenses", get(list_licenses_handler))
        .route("/api/v1/licenses/batch", post(batch_create_license_handler))
        .route("/api/v1/licenses/:license_id", get(get_license_handler))
        .route(
            "/api/v1/licenses/:license_id",
            patch(update_license_handler),
        );

    router.with_state(state)
}
