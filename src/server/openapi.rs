//! OpenAPI documentation for the Talos API.
//!
//! This module provides OpenAPI 3.0 specification generation using utoipa,
//! along with Swagger UI for interactive API exploration.
//!
//! # Usage
//!
//! Enable the `openapi` feature and add the Swagger UI routes:
//!
//! ```rust,ignore
//! use talos::server::openapi::ApiDoc;
//! use utoipa_swagger_ui::SwaggerUi;
//!
//! let app = Router::new()
//!     // ... your routes ...
//!     .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));
//! ```

use utoipa::OpenApi;

/// OpenAPI documentation for the Talos Licensing API.
///
/// This struct generates the OpenAPI specification for all Talos endpoints.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Talos Licensing API",
        version = "1.0.0",
        description = "A secure, hardware-bound licensing system for software applications.",
        license(name = "MIT", url = "https://opensource.org/licenses/MIT"),
        contact(name = "Talos", url = "https://github.com/dmriding/talos")
    ),
    servers(
        (url = "/", description = "Local server")
    ),
    tags(
        (name = "client", description = "Client endpoints for license validation and binding"),
        (name = "admin", description = "Admin endpoints for license management (requires authentication)"),
        (name = "tokens", description = "API token management endpoints"),
        (name = "legacy", description = "Legacy endpoints for backwards compatibility")
    ),
    paths(
        // Client API endpoints
        crate::server::client_api::bind_handler,
        crate::server::client_api::release_handler,
        crate::server::client_api::validate_handler,
        crate::server::client_api::validate_or_bind_handler,
        crate::server::client_api::client_heartbeat_handler,
        crate::server::client_api::validate_feature_handler,
        // Legacy endpoints
        crate::server::handlers::activate_license_handler,
        crate::server::handlers::validate_license_handler,
        crate::server::handlers::deactivate_license_handler,
        crate::server::handlers::heartbeat_handler,
    ),
    components(
        schemas(
            // Client API schemas
            crate::server::client_api::BindRequest,
            crate::server::client_api::BindResponse,
            crate::server::client_api::ReleaseRequest,
            crate::server::client_api::ReleaseResponse,
            crate::server::client_api::ValidateRequest,
            crate::server::client_api::ValidateResponse,
            crate::server::client_api::ValidateOrBindRequest,
            crate::server::client_api::ClientHeartbeatRequest,
            crate::server::client_api::ClientHeartbeatResponse,
            crate::server::client_api::ValidateFeatureRequest,
            crate::server::client_api::ValidateFeatureResponse,
            crate::server::client_api::ClientError,
            crate::server::client_api::ClientErrorCode,
            // Legacy handler schemas
            crate::server::handlers::LicenseRequest,
            crate::server::handlers::LicenseResponse,
            crate::server::handlers::HeartbeatRequest,
            crate::server::handlers::HeartbeatResponse,
            // Token schemas
            crate::server::tokens::CreateTokenRequest,
            crate::server::tokens::CreateTokenResponse,
            crate::server::tokens::TokenMetadata,
            crate::server::tokens::ListTokensResponse,
            crate::server::tokens::TokenResponse,
            crate::server::tokens::RevokeTokenResponse,
            crate::server::tokens::TokenErrorResponse,
        )
    )
)]
pub struct ApiDoc;

/// OpenAPI documentation including admin endpoints.
///
/// This includes all endpoints from `ApiDoc` plus admin-only endpoints
/// that require the `admin-api` feature.
#[cfg(feature = "admin-api")]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Talos Licensing API",
        version = "1.0.0",
        description = "A secure, hardware-bound licensing system for software applications.",
        license(name = "MIT", url = "https://opensource.org/licenses/MIT"),
        contact(name = "Talos", url = "https://github.com/dmriding/talos")
    ),
    servers(
        (url = "/", description = "Local server")
    ),
    tags(
        (name = "client", description = "Client endpoints for license validation and binding"),
        (name = "admin", description = "Admin endpoints for license management (requires authentication)"),
        (name = "tokens", description = "API token management endpoints"),
        (name = "legacy", description = "Legacy endpoints for backwards compatibility")
    ),
    paths(
        // Client API endpoints
        crate::server::client_api::bind_handler,
        crate::server::client_api::release_handler,
        crate::server::client_api::validate_handler,
        crate::server::client_api::validate_or_bind_handler,
        crate::server::client_api::client_heartbeat_handler,
        crate::server::client_api::validate_feature_handler,
        // Legacy endpoints
        crate::server::handlers::activate_license_handler,
        crate::server::handlers::validate_license_handler,
        crate::server::handlers::deactivate_license_handler,
        crate::server::handlers::heartbeat_handler,
        // Admin API endpoints
        crate::server::admin::create_license_handler,
        crate::server::admin::batch_create_license_handler,
        crate::server::admin::get_license_handler,
        crate::server::admin::list_licenses_handler,
        crate::server::admin::update_license_handler,
        crate::server::admin::revoke_license_handler,
        crate::server::admin::reinstate_license_handler,
        crate::server::admin::extend_license_handler,
        crate::server::admin::update_usage_handler,
        crate::server::admin::admin_release_handler,
        crate::server::admin::blacklist_license_handler,
        // Token endpoints
        crate::server::tokens::create_token_handler,
        crate::server::tokens::list_tokens_handler,
        crate::server::tokens::get_token_handler,
        crate::server::tokens::revoke_token_handler,
    ),
    components(
        schemas(
            // Client API schemas
            crate::server::client_api::BindRequest,
            crate::server::client_api::BindResponse,
            crate::server::client_api::ReleaseRequest,
            crate::server::client_api::ReleaseResponse,
            crate::server::client_api::ValidateRequest,
            crate::server::client_api::ValidateResponse,
            crate::server::client_api::ValidateOrBindRequest,
            crate::server::client_api::ClientHeartbeatRequest,
            crate::server::client_api::ClientHeartbeatResponse,
            crate::server::client_api::ValidateFeatureRequest,
            crate::server::client_api::ValidateFeatureResponse,
            crate::server::client_api::ClientError,
            crate::server::client_api::ClientErrorCode,
            // Legacy handler schemas
            crate::server::handlers::LicenseRequest,
            crate::server::handlers::LicenseResponse,
            crate::server::handlers::HeartbeatRequest,
            crate::server::handlers::HeartbeatResponse,
            // Admin API schemas
            crate::server::admin::CreateLicenseRequest,
            crate::server::admin::LicenseResponse,
            crate::server::admin::BatchCreateLicenseRequest,
            crate::server::admin::BatchCreateResponse,
            crate::server::admin::LicenseSummary,
            crate::server::admin::ListLicensesResponse,
            crate::server::admin::UpdateLicenseRequest,
            crate::server::admin::RevokeLicenseRequest,
            crate::server::admin::RevokeLicenseResponse,
            crate::server::admin::ReinstateLicenseRequest,
            crate::server::admin::ReinstateLicenseResponse,
            crate::server::admin::ExtendLicenseRequest,
            crate::server::admin::ExtendLicenseResponse,
            crate::server::admin::UpdateUsageRequest,
            crate::server::admin::UpdateUsageResponse,
            crate::server::admin::AdminReleaseRequest,
            crate::server::admin::AdminReleaseResponse,
            crate::server::admin::BlacklistLicenseRequest,
            crate::server::admin::BlacklistLicenseResponse,
            // Token schemas
            crate::server::tokens::CreateTokenRequest,
            crate::server::tokens::CreateTokenResponse,
            crate::server::tokens::TokenMetadata,
            crate::server::tokens::ListTokensResponse,
            crate::server::tokens::TokenResponse,
            crate::server::tokens::RevokeTokenResponse,
            crate::server::tokens::TokenErrorResponse,
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDocWithAdmin;

/// Security scheme modifier for JWT authentication.
#[cfg(feature = "admin-api")]
struct SecurityAddon;

#[cfg(feature = "admin-api")]
impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::Http::new(
                        utoipa::openapi::security::HttpAuthScheme::Bearer,
                    ),
                ),
            );
        }
    }
}

/// Get the appropriate OpenAPI document based on enabled features.
///
/// Returns `ApiDocWithAdmin` if `admin-api` feature is enabled,
/// otherwise returns `ApiDoc`.
#[cfg(feature = "admin-api")]
pub fn get_openapi() -> utoipa::openapi::OpenApi {
    ApiDocWithAdmin::openapi()
}

#[cfg(not(feature = "admin-api"))]
pub fn get_openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
