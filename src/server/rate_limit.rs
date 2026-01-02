//! Rate limiting middleware for Talos public endpoints.
//!
//! This module provides rate limiting to protect public endpoints from brute force attacks.
//! Rate limits are configurable via `config.toml` or environment variables.
//!
//! # Configuration
//!
//! ```toml
//! [rate_limit]
//! enabled = true
//! validate_rpm = 100   # /validate endpoint: 100 requests per minute
//! heartbeat_rpm = 60   # /heartbeat endpoint: 60 requests per minute
//! bind_rpm = 10        # /bind and /release endpoints: 10 requests per minute
//! burst_size = 5       # Allow short bursts above the limit
//! ```
//!
//! # Features
//!
//! - Per-IP rate limiting using the client's IP address
//! - Configurable limits per endpoint type
//! - Burst allowance for legitimate traffic spikes
//! - Returns 429 Too Many Requests with Retry-After header
//!
//! # Usage
//!
//! Use `SmartIpKeyExtractor` which automatically handles X-Forwarded-For headers
//! for proxied requests, or use the built-in `PeerIpKeyExtractor` for direct connections.

use axum::{
    body::Body,
    http::StatusCode,
    response::Response,
};
use governor::middleware::NoOpMiddleware;
use std::sync::Arc;
use tower_governor::governor::GovernorConfigBuilder;

pub use tower_governor::key_extractor::SmartIpKeyExtractor;
pub use tower_governor::GovernorLayer;

use crate::config::RateLimitConfig;

/// Rate limiter types for different endpoint categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitType {
    /// For /validate endpoint (higher limit)
    Validate,
    /// For /heartbeat endpoint (medium limit)
    Heartbeat,
    /// For /bind and /release endpoints (lower limit)
    Bind,
}

/// Create a rate limiting layer for the specified endpoint type.
///
/// Returns a `GovernorLayer` configured with the appropriate limits from config.
/// Uses `SmartIpKeyExtractor` which checks X-Forwarded-For, X-Real-IP headers
/// before falling back to the peer IP address.
///
/// # Important
///
/// When using this with Axum, you must create your server with:
/// ```ignore
/// .into_make_service_with_connect_info::<SocketAddr>()
/// ```
/// instead of `.into_make_service()` for IP extraction to work correctly.
pub fn create_rate_limiter(
    config: &RateLimitConfig,
    limit_type: RateLimitType,
) -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware> {
    let rpm = match limit_type {
        RateLimitType::Validate => config.validate_rpm,
        RateLimitType::Heartbeat => config.heartbeat_rpm,
        RateLimitType::Bind => config.bind_rpm,
    };

    // Convert RPM to a replenish interval
    // requests_per_minute -> one request every (60000/rpm) milliseconds
    let interval_ms = if rpm > 0 { 60_000 / rpm } else { 60_000 };

    let governor_config = GovernorConfigBuilder::default()
        .per_millisecond(interval_ms.into())
        .burst_size(config.burst_size)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .expect("failed to build governor config");

    GovernorLayer {
        config: Arc::new(governor_config),
    }
}

/// Custom error response for rate limiting.
///
/// Returns a 429 status code with a JSON error body and Retry-After header.
pub fn rate_limit_error_response(retry_after_secs: u64) -> Response<Body> {
    let retry_after = retry_after_secs.max(1);
    let body = serde_json::json!({
        "error": "Too many requests",
        "message": format!("Rate limit exceeded. Please retry after {} seconds.", retry_after),
        "retry_after_seconds": retry_after
    });

    Response::builder()
        .status(StatusCode::TOO_MANY_REQUESTS)
        .header("Content-Type", "application/json")
        .header("Retry-After", retry_after.to_string())
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_config_defaults() {
        let config = RateLimitConfig::default();
        assert!(config.enabled);
        assert_eq!(config.validate_rpm, 100);
        assert_eq!(config.heartbeat_rpm, 60);
        assert_eq!(config.bind_rpm, 10);
        assert_eq!(config.burst_size, 5);
    }

    #[test]
    fn rate_limit_error_response_format() {
        let response = rate_limit_error_response(30);
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            response.headers().get("Retry-After").unwrap().to_str().unwrap(),
            "30"
        );
    }

    #[test]
    fn rate_limit_error_minimum_retry_after() {
        // Even for zero wait, minimum retry-after should be 1 second
        let response = rate_limit_error_response(0);
        assert_eq!(
            response.headers().get("Retry-After").unwrap().to_str().unwrap(),
            "1"
        );
    }

    #[test]
    fn create_rate_limiter_validate() {
        let config = RateLimitConfig::default();
        let _layer = create_rate_limiter(&config, RateLimitType::Validate);
        // Just verify it doesn't panic
    }

    #[test]
    fn create_rate_limiter_heartbeat() {
        let config = RateLimitConfig::default();
        let _layer = create_rate_limiter(&config, RateLimitType::Heartbeat);
    }

    #[test]
    fn create_rate_limiter_bind() {
        let config = RateLimitConfig::default();
        let _layer = create_rate_limiter(&config, RateLimitType::Bind);
    }
}
