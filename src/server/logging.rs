//! Request logging middleware for Talos.
//!
//! This module provides structured logging for all API requests including:
//! - Unique request ID tracking
//! - Request timing
//! - Method, path, and status logging
//! - Request ID propagation in response headers
//!
//! # Usage
//!
//! ```rust,ignore
//! use talos::server::logging::RequestLoggingLayer;
//!
//! let app = Router::new()
//!     .route("/api/v1/health", get(health_handler))
//!     .layer(RequestLoggingLayer::new());
//! ```

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, Response},
    middleware::Next,
};
use std::time::Instant;
use tracing::{info, info_span, warn, Instrument};
use uuid::Uuid;

/// License state change event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseEvent {
    /// License was created
    Created,
    /// License was bound to hardware
    Bound,
    /// License was released from hardware
    Released,
    /// License was validated successfully
    Validated,
    /// License validation failed
    ValidationFailed,
    /// License was activated (legacy)
    Activated,
    /// License was deactivated (legacy)
    Deactivated,
    /// License was revoked
    Revoked,
    /// License was reinstated
    Reinstated,
    /// License was suspended
    Suspended,
    /// License was extended
    Extended,
    /// License was blacklisted
    Blacklisted,
    /// License heartbeat received
    Heartbeat,
    /// License usage updated
    UsageUpdated,
}

impl std::fmt::Display for LicenseEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LicenseEvent::Created => "created",
            LicenseEvent::Bound => "bound",
            LicenseEvent::Released => "released",
            LicenseEvent::Validated => "validated",
            LicenseEvent::ValidationFailed => "validation_failed",
            LicenseEvent::Activated => "activated",
            LicenseEvent::Deactivated => "deactivated",
            LicenseEvent::Revoked => "revoked",
            LicenseEvent::Reinstated => "reinstated",
            LicenseEvent::Suspended => "suspended",
            LicenseEvent::Extended => "extended",
            LicenseEvent::Blacklisted => "blacklisted",
            LicenseEvent::Heartbeat => "heartbeat",
            LicenseEvent::UsageUpdated => "usage_updated",
        };
        write!(f, "{}", s)
    }
}

/// Log a license state change event.
///
/// This function logs structured information about license state changes
/// for audit and debugging purposes.
///
/// # Arguments
///
/// * `event` - The type of license event
/// * `license_id` - The license ID (or license key)
/// * `details` - Optional additional details about the event
pub fn log_license_event(event: LicenseEvent, license_id: &str, details: Option<&str>) {
    let span = info_span!(
        "license_event",
        event = %event,
        license_id = %license_id,
    );
    let _enter = span.enter();

    match event {
        LicenseEvent::ValidationFailed => {
            if let Some(d) = details {
                warn!(reason = %d, "License event occurred");
            } else {
                warn!("License event occurred");
            }
        }
        _ => {
            if let Some(d) = details {
                info!(details = %d, "License event occurred");
            } else {
                info!("License event occurred");
            }
        }
    }
}

/// Log a license state change with hardware binding information.
///
/// # Arguments
///
/// * `event` - The type of license event
/// * `license_id` - The license ID (or license key)
/// * `hardware_id` - The hardware ID involved in the operation
/// * `device_name` - Optional device name
pub fn log_license_binding_event(
    event: LicenseEvent,
    license_id: &str,
    hardware_id: &str,
    device_name: Option<&str>,
) {
    let span = info_span!(
        "license_binding",
        event = %event,
        license_id = %license_id,
        hardware_id = %hardware_id,
    );
    let _enter = span.enter();

    if let Some(name) = device_name {
        info!(device_name = %name, "License binding event occurred");
    } else {
        info!("License binding event occurred");
    }
}

/// Header name for the request ID.
pub const REQUEST_ID_HEADER: &str = "X-Request-Id";

/// Generate a new unique request ID.
pub fn generate_request_id() -> String {
    Uuid::new_v4().to_string()
}

/// Logging middleware that tracks request timing and generates request IDs.
///
/// This middleware:
/// 1. Generates a unique request ID for each incoming request
/// 2. Creates a tracing span with the request ID
/// 3. Logs the request method and path
/// 4. Measures and logs the response time
/// 5. Adds the request ID to the response headers
pub async fn request_logging_middleware(request: Request, next: Next) -> Response<Body> {
    let request_id = generate_request_id();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();

    // Create a span for this request
    let span = info_span!(
        "request",
        request_id = %request_id,
        method = %method,
        path = %path,
    );

    let start = Instant::now();

    // Process the request within the span
    let response = async move {
        info!("Started processing request");
        let response = next.run(request).await;
        response
    }
    .instrument(span.clone())
    .await;

    let duration = start.elapsed();
    let status = response.status();

    // Log completion
    let _enter = span.enter();
    info!(
        status = %status.as_u16(),
        duration_ms = %duration.as_millis(),
        "Request completed"
    );

    // Add request ID to response headers
    let (mut parts, body) = response.into_parts();
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        parts.headers.insert(REQUEST_ID_HEADER, header_value);
    }

    Response::from_parts(parts, body)
}

/// Health check response structure.
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HealthResponse {
    /// Service status ("healthy" or "unhealthy")
    pub status: String,
    /// Service name
    pub service: String,
    /// Service version
    pub version: String,
    /// Database connectivity status
    pub database: DatabaseHealth,
}

/// Database health status.
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DatabaseHealth {
    /// Whether the database is connected
    pub connected: bool,
    /// Database type (sqlite or postgres)
    pub db_type: String,
}

impl HealthResponse {
    /// Create a healthy response.
    pub fn healthy(db_connected: bool, db_type: &str) -> Self {
        Self {
            status: if db_connected { "healthy" } else { "degraded" }.to_string(),
            service: "talos".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            database: DatabaseHealth {
                connected: db_connected,
                db_type: db_type.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_id_is_valid_uuid() {
        let id = generate_request_id();
        assert!(Uuid::parse_str(&id).is_ok());
    }

    #[test]
    fn health_response_healthy() {
        let health = HealthResponse::healthy(true, "sqlite");
        assert_eq!(health.status, "healthy");
        assert_eq!(health.service, "talos");
        assert!(health.database.connected);
    }

    #[test]
    fn health_response_degraded() {
        let health = HealthResponse::healthy(false, "postgres");
        assert_eq!(health.status, "degraded");
        assert!(!health.database.connected);
    }
}
