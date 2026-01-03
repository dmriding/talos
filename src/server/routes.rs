use axum::{routing::post, Router};

#[cfg(feature = "admin-api")]
use axum::routing::{get, patch};

use crate::server::client_api::{
    bind_handler, client_heartbeat_handler, release_handler, validate_feature_handler,
    validate_handler, validate_or_bind_handler,
};
use crate::server::handlers::{
    activate_license_handler, deactivate_license_handler, heartbeat_handler,
    validate_license_handler, AppState,
};

#[cfg(feature = "admin-api")]
use crate::server::admin::{
    admin_release_handler, batch_create_license_handler, create_license_handler,
    extend_license_handler, get_license_handler, list_licenses_handler, reinstate_license_handler,
    revoke_license_handler, update_license_handler, update_usage_handler,
};

/// Build the main application router for the Talos server.
///
/// This is a convenience helper so `main.rs` or tests can
/// construct the router in a single call.
///
/// # Routes
///
/// ## Legacy client endpoints (for backwards compatibility)
/// - `POST /activate` - Activate a license
/// - `POST /validate` - Validate a license
/// - `POST /deactivate` - Deactivate a license
/// - `POST /heartbeat` - Send heartbeat
///
/// ## New client endpoints (v1 API)
/// - `POST /api/v1/client/bind` - Bind license to hardware
/// - `POST /api/v1/client/release` - Release license from hardware
/// - `POST /api/v1/client/validate` - Validate a license
/// - `POST /api/v1/client/validate-or-bind` - Validate or auto-bind
/// - `POST /api/v1/client/heartbeat` - Send heartbeat
/// - `POST /api/v1/client/validate-feature` - Validate a specific feature
///
/// ## Admin endpoints (requires `admin-api` feature)
/// - `POST /api/v1/licenses` - Create a license
/// - `POST /api/v1/licenses/batch` - Batch create licenses
/// - `GET /api/v1/licenses/{license_id}` - Get a license
/// - `GET /api/v1/licenses` - List licenses (requires org_id query param)
/// - `PATCH /api/v1/licenses/{license_id}` - Update a license
/// - `POST /api/v1/licenses/{license_id}/release` - Admin force release
/// - `POST /api/v1/licenses/{license_id}/revoke` - Revoke a license
/// - `POST /api/v1/licenses/{license_id}/reinstate` - Reinstate a revoked/suspended license
/// - `POST /api/v1/licenses/{license_id}/extend` - Extend license expiration
/// - `PATCH /api/v1/licenses/{license_id}/usage` - Update bandwidth/usage tracking
pub fn build_router(state: AppState) -> Router {
    let router = Router::new()
        // Legacy client endpoints (backwards compatibility)
        .route("/activate", post(activate_license_handler))
        .route("/validate", post(validate_license_handler))
        .route("/deactivate", post(deactivate_license_handler))
        .route("/heartbeat", post(heartbeat_handler))
        // New client API v1 endpoints
        .route("/api/v1/client/bind", post(bind_handler))
        .route("/api/v1/client/release", post(release_handler))
        .route("/api/v1/client/validate", post(validate_handler))
        .route(
            "/api/v1/client/validate-or-bind",
            post(validate_or_bind_handler),
        )
        .route("/api/v1/client/heartbeat", post(client_heartbeat_handler))
        .route(
            "/api/v1/client/validate-feature",
            post(validate_feature_handler),
        );

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
        )
        .route(
            "/api/v1/licenses/:license_id/release",
            post(admin_release_handler),
        )
        .route(
            "/api/v1/licenses/:license_id/revoke",
            post(revoke_license_handler),
        )
        .route(
            "/api/v1/licenses/:license_id/reinstate",
            post(reinstate_license_handler),
        )
        .route(
            "/api/v1/licenses/:license_id/extend",
            post(extend_license_handler),
        )
        .route(
            "/api/v1/licenses/:license_id/usage",
            patch(update_usage_handler),
        );

    router.with_state(state)
}
