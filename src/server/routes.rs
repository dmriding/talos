use axum::{middleware, routing::get, routing::post, Router};

#[cfg(feature = "admin-api")]
use axum::routing::{delete, patch};

#[cfg(feature = "openapi")]
use utoipa_swagger_ui::SwaggerUi;

use crate::server::client_api::{
    bind_handler, client_heartbeat_handler, release_handler, validate_feature_handler,
    validate_handler, validate_or_bind_handler,
};
use crate::server::handlers::{
    activate_license_handler, deactivate_license_handler, health_handler, heartbeat_handler,
    validate_license_handler, AppState,
};
use crate::server::logging::request_logging_middleware;

#[cfg(feature = "admin-api")]
use crate::config::get_config;
#[cfg(feature = "admin-api")]
use crate::server::ip_whitelist::IpWhitelistLayer;

#[cfg(feature = "admin-api")]
use crate::server::admin::{
    admin_release_handler, batch_create_license_handler, blacklist_license_handler,
    create_license_handler, extend_license_handler, get_license_handler, list_licenses_handler,
    reinstate_license_handler, revoke_license_handler, update_license_handler,
    update_usage_handler,
};

#[cfg(feature = "admin-api")]
use crate::server::tokens::{
    create_token_handler, get_token_handler, list_tokens_handler, revoke_token_handler,
};

#[cfg(all(feature = "admin-api", feature = "jwt-auth"))]
use crate::server::auth::AuthLayer;

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
/// - `POST /api/v1/licenses/{license_id}/blacklist` - Permanently blacklist a license
///
/// ## Token endpoints (requires `admin-api` feature)
/// - `POST /api/v1/tokens` - Create a new API token
/// - `GET /api/v1/tokens` - List all API tokens
/// - `GET /api/v1/tokens/{token_id}` - Get a specific token
/// - `DELETE /api/v1/tokens/{token_id}` - Revoke a token
pub fn build_router(state: AppState) -> Router {
    let router = Router::new()
        // Health check endpoint (no auth required)
        .route("/health", get(health_handler))
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
    // When both admin-api and jwt-auth are enabled, apply auth + IP whitelist middleware
    #[cfg(all(feature = "admin-api", feature = "jwt-auth"))]
    let router = {
        // Get IP whitelist from config (if configured)
        let ip_whitelist_layer = get_config()
            .map(|c| IpWhitelistLayer::from_config(&c.admin.ip_whitelist))
            .unwrap_or_else(|_| IpWhitelistLayer::from_config(&[]));

        // Build admin routes as a nested router with auth + IP whitelist layers
        let admin_routes = Router::new()
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
            )
            .route(
                "/api/v1/licenses/:license_id/blacklist",
                post(blacklist_license_handler),
            )
            // Token management routes
            .route("/api/v1/tokens", post(create_token_handler))
            .route("/api/v1/tokens", get(list_tokens_handler))
            .route("/api/v1/tokens/:token_id", get(get_token_handler))
            .route("/api/v1/tokens/:token_id", delete(revoke_token_handler))
            // Apply IP whitelist first (outer layer), then auth (inner layer)
            .layer(AuthLayer::new(state.auth.clone()))
            .layer(ip_whitelist_layer);

        router.merge(admin_routes)
    };

    // Admin API without JWT auth (admin-api only, no jwt-auth)
    // Still applies IP whitelist for security
    #[cfg(all(feature = "admin-api", not(feature = "jwt-auth")))]
    let router = {
        // Get IP whitelist from config (if configured)
        let ip_whitelist_layer = get_config()
            .map(|c| IpWhitelistLayer::from_config(&c.admin.ip_whitelist))
            .unwrap_or_else(|_| IpWhitelistLayer::from_config(&[]));

        let admin_routes = Router::new()
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
            )
            .route(
                "/api/v1/licenses/:license_id/blacklist",
                post(blacklist_license_handler),
            )
            // Token management routes
            .route("/api/v1/tokens", post(create_token_handler))
            .route("/api/v1/tokens", get(list_tokens_handler))
            .route("/api/v1/tokens/:token_id", get(get_token_handler))
            .route("/api/v1/tokens/:token_id", delete(revoke_token_handler))
            .layer(ip_whitelist_layer);

        router.merge(admin_routes)
    };

    // Add Swagger UI routes if openapi feature is enabled
    #[cfg(feature = "openapi")]
    let router = {
        use crate::server::openapi::get_openapi;
        router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", get_openapi()))
    };

    // Add request logging middleware to all routes
    router
        .layer(middleware::from_fn(request_logging_middleware))
        .with_state(state)
}
