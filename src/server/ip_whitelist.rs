//! IP Whitelist middleware for admin API protection.
//!
//! This module provides middleware that restricts access to admin endpoints
//! based on the client's IP address. It supports both individual IPs and
//! CIDR notation for network ranges.
//!
//! # Configuration
//!
//! ```toml
//! [admin]
//! ip_whitelist = ["127.0.0.1", "10.0.0.0/8", "192.168.0.0/16"]
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use talos::server::ip_whitelist::{IpWhitelistLayer, IpWhitelist};
//!
//! let whitelist = IpWhitelist::new(&["127.0.0.1", "10.0.0.0/8"]);
//! let admin_routes = Router::new()
//!     .route("/api/v1/licenses", post(create_license_handler))
//!     .layer(IpWhitelistLayer::new(whitelist));
//! ```

use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
};
use serde_json::json;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::pin::Pin;
use std::str::FromStr;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::warn;

/// Represents a CIDR network range or a single IP address.
#[derive(Debug, Clone)]
pub enum IpNetwork {
    /// A single IP address (v4 or v6)
    Single(IpAddr),
    /// An IPv4 network with prefix length
    V4Network { addr: Ipv4Addr, prefix_len: u8 },
    /// An IPv6 network with prefix length
    V6Network { addr: Ipv6Addr, prefix_len: u8 },
}

impl IpNetwork {
    /// Parse an IP address or CIDR notation string.
    ///
    /// Supports:
    /// - `"192.168.1.1"` - Single IPv4 address
    /// - `"::1"` - Single IPv6 address
    /// - `"10.0.0.0/8"` - IPv4 CIDR
    /// - `"fd00::/8"` - IPv6 CIDR
    pub fn parse(s: &str) -> Option<Self> {
        if let Some((addr_str, prefix_str)) = s.split_once('/') {
            let prefix_len: u8 = prefix_str.parse().ok()?;

            if let Ok(v4) = Ipv4Addr::from_str(addr_str) {
                if prefix_len > 32 {
                    return None;
                }
                return Some(IpNetwork::V4Network {
                    addr: v4,
                    prefix_len,
                });
            }

            if let Ok(v6) = Ipv6Addr::from_str(addr_str) {
                if prefix_len > 128 {
                    return None;
                }
                return Some(IpNetwork::V6Network {
                    addr: v6,
                    prefix_len,
                });
            }

            None
        } else {
            // Single IP address
            IpAddr::from_str(s).ok().map(IpNetwork::Single)
        }
    }

    /// Check if the given IP address is contained in this network.
    pub fn contains(&self, ip: &IpAddr) -> bool {
        match (self, ip) {
            (IpNetwork::Single(allowed), ip) => allowed == ip,

            (IpNetwork::V4Network { addr, prefix_len }, IpAddr::V4(ip)) => {
                let mask = if *prefix_len == 0 {
                    0
                } else {
                    !0u32 << (32 - prefix_len)
                };
                let network = u32::from(*addr) & mask;
                let ip_masked = u32::from(*ip) & mask;
                network == ip_masked
            }

            (IpNetwork::V6Network { addr, prefix_len }, IpAddr::V6(ip)) => {
                let addr_bits = u128::from(*addr);
                let ip_bits = u128::from(*ip);
                let mask = if *prefix_len == 0 {
                    0
                } else {
                    !0u128 << (128 - prefix_len)
                };
                (addr_bits & mask) == (ip_bits & mask)
            }

            // IPv4 network can't contain IPv6 address and vice versa
            _ => false,
        }
    }
}

/// IP whitelist configuration.
///
/// Holds a list of allowed IP addresses and networks.
#[derive(Debug, Clone, Default)]
pub struct IpWhitelist {
    networks: Vec<IpNetwork>,
    enabled: bool,
}

impl IpWhitelist {
    /// Create a new IP whitelist from a list of IP/CIDR strings.
    ///
    /// Invalid entries are logged and skipped.
    pub fn new(entries: &[String]) -> Self {
        if entries.is_empty() {
            return Self {
                networks: Vec::new(),
                enabled: false,
            };
        }

        let networks: Vec<IpNetwork> = entries
            .iter()
            .filter_map(|s| {
                let trimmed = s.trim();
                match IpNetwork::parse(trimmed) {
                    Some(net) => Some(net),
                    None => {
                        warn!("Invalid IP whitelist entry ignored: {}", trimmed);
                        None
                    }
                }
            })
            .collect();

        Self {
            networks,
            enabled: true,
        }
    }

    /// Check if an IP address is allowed by this whitelist.
    ///
    /// Returns `true` if:
    /// - The whitelist is disabled (empty), OR
    /// - The IP matches any entry in the whitelist
    pub fn is_allowed(&self, ip: &IpAddr) -> bool {
        if !self.enabled {
            return true;
        }
        self.networks.iter().any(|net| net.contains(ip))
    }

    /// Check if the whitelist is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Layer for applying IP whitelist middleware.
#[derive(Clone)]
pub struct IpWhitelistLayer {
    whitelist: IpWhitelist,
}

impl IpWhitelistLayer {
    /// Create a new IP whitelist layer.
    pub fn new(whitelist: IpWhitelist) -> Self {
        Self { whitelist }
    }

    /// Create from config entries.
    pub fn from_config(entries: &[String]) -> Self {
        Self {
            whitelist: IpWhitelist::new(entries),
        }
    }
}

impl<S> Layer<S> for IpWhitelistLayer {
    type Service = IpWhitelistMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        IpWhitelistMiddleware {
            inner,
            whitelist: self.whitelist.clone(),
        }
    }
}

/// Middleware service that checks IP addresses against a whitelist.
#[derive(Clone)]
pub struct IpWhitelistMiddleware<S> {
    inner: S,
    whitelist: IpWhitelist,
}

impl<S> Service<Request<Body>> for IpWhitelistMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        // Skip check if whitelist is disabled
        if !self.whitelist.is_enabled() {
            return Box::pin(self.inner.call(req));
        }

        // Extract client IP from request
        let client_ip = extract_client_ip(&req);

        match client_ip {
            Some(ip) if self.whitelist.is_allowed(&ip) => {
                // IP is whitelisted, proceed
                Box::pin(self.inner.call(req))
            }
            Some(ip) => {
                // IP is not whitelisted, reject
                warn!(
                    ip = %ip,
                    path = %req.uri().path(),
                    "Admin API access blocked: IP not in whitelist"
                );
                Box::pin(async move { Ok(ip_blocked_response()) })
            }
            None => {
                // Could not determine client IP
                warn!(
                    path = %req.uri().path(),
                    "Admin API access blocked: could not determine client IP"
                );
                Box::pin(async move { Ok(ip_blocked_response()) })
            }
        }
    }
}

/// Extract the client IP address from the request.
///
/// Checks in order:
/// 1. `X-Forwarded-For` header (first IP in the list)
/// 2. `X-Real-IP` header
/// 3. Connection info (if available)
fn extract_client_ip<B>(req: &Request<B>) -> Option<IpAddr> {
    // Try X-Forwarded-For header first (common with reverse proxies)
    if let Some(xff) = req.headers().get("x-forwarded-for") {
        if let Ok(xff_str) = xff.to_str() {
            // Take the first IP in the chain (original client)
            if let Some(first_ip) = xff_str.split(',').next() {
                if let Ok(ip) = IpAddr::from_str(first_ip.trim()) {
                    return Some(ip);
                }
            }
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = IpAddr::from_str(ip_str.trim()) {
                return Some(ip);
            }
        }
    }

    // Try to get from connection info extension (set by axum/hyper)
    // This is typically only available when not behind a proxy
    if let Some(connect_info) = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
    {
        return Some(connect_info.0.ip());
    }

    None
}

/// Generate a 403 Forbidden response for blocked IPs.
fn ip_blocked_response() -> Response<Body> {
    let body = json!({
        "error": {
            "code": "IP_NOT_ALLOWED",
            "message": "Your IP address is not authorized to access this endpoint"
        }
    });

    (
        StatusCode::FORBIDDEN,
        [("content-type", "application/json")],
        body.to_string(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_ipv4() {
        let net = IpNetwork::parse("192.168.1.1").unwrap();
        assert!(matches!(net, IpNetwork::Single(IpAddr::V4(_))));
    }

    #[test]
    fn parse_single_ipv6() {
        let net = IpNetwork::parse("::1").unwrap();
        assert!(matches!(net, IpNetwork::Single(IpAddr::V6(_))));
    }

    #[test]
    fn parse_ipv4_cidr() {
        let net = IpNetwork::parse("10.0.0.0/8").unwrap();
        assert!(matches!(net, IpNetwork::V4Network { prefix_len: 8, .. }));
    }

    #[test]
    fn parse_ipv6_cidr() {
        let net = IpNetwork::parse("fd00::/8").unwrap();
        assert!(matches!(net, IpNetwork::V6Network { prefix_len: 8, .. }));
    }

    #[test]
    fn parse_invalid() {
        assert!(IpNetwork::parse("invalid").is_none());
        assert!(IpNetwork::parse("192.168.1.1/33").is_none()); // Invalid prefix
        assert!(IpNetwork::parse("192.168.1.1/abc").is_none());
    }

    #[test]
    fn single_ip_contains() {
        let net = IpNetwork::parse("192.168.1.100").unwrap();
        assert!(net.contains(&"192.168.1.100".parse().unwrap()));
        assert!(!net.contains(&"192.168.1.101".parse().unwrap()));
    }

    #[test]
    fn ipv4_cidr_contains() {
        let net = IpNetwork::parse("10.0.0.0/8").unwrap();
        assert!(net.contains(&"10.0.0.1".parse().unwrap()));
        assert!(net.contains(&"10.255.255.255".parse().unwrap()));
        assert!(!net.contains(&"11.0.0.1".parse().unwrap()));
        assert!(!net.contains(&"192.168.1.1".parse().unwrap()));
    }

    #[test]
    fn ipv4_cidr_24_contains() {
        let net = IpNetwork::parse("192.168.1.0/24").unwrap();
        assert!(net.contains(&"192.168.1.1".parse().unwrap()));
        assert!(net.contains(&"192.168.1.255".parse().unwrap()));
        assert!(!net.contains(&"192.168.2.1".parse().unwrap()));
    }

    #[test]
    fn ipv4_cidr_16_contains() {
        let net = IpNetwork::parse("192.168.0.0/16").unwrap();
        assert!(net.contains(&"192.168.0.1".parse().unwrap()));
        assert!(net.contains(&"192.168.255.255".parse().unwrap()));
        assert!(!net.contains(&"192.169.0.1".parse().unwrap()));
    }

    #[test]
    fn whitelist_empty_allows_all() {
        let whitelist = IpWhitelist::new(&[]);
        assert!(!whitelist.is_enabled());
        assert!(whitelist.is_allowed(&"192.168.1.1".parse().unwrap()));
        assert!(whitelist.is_allowed(&"10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn whitelist_single_ip() {
        let whitelist = IpWhitelist::new(&["192.168.1.100".to_string()]);
        assert!(whitelist.is_enabled());
        assert!(whitelist.is_allowed(&"192.168.1.100".parse().unwrap()));
        assert!(!whitelist.is_allowed(&"192.168.1.101".parse().unwrap()));
    }

    #[test]
    fn whitelist_cidr() {
        let whitelist = IpWhitelist::new(&["10.0.0.0/8".to_string()]);
        assert!(whitelist.is_allowed(&"10.0.0.1".parse().unwrap()));
        assert!(whitelist.is_allowed(&"10.255.255.255".parse().unwrap()));
        assert!(!whitelist.is_allowed(&"192.168.1.1".parse().unwrap()));
    }

    #[test]
    fn whitelist_multiple_entries() {
        let whitelist = IpWhitelist::new(&[
            "127.0.0.1".to_string(),
            "10.0.0.0/8".to_string(),
            "192.168.0.0/16".to_string(),
        ]);
        assert!(whitelist.is_allowed(&"127.0.0.1".parse().unwrap()));
        assert!(whitelist.is_allowed(&"10.5.5.5".parse().unwrap()));
        assert!(whitelist.is_allowed(&"192.168.100.50".parse().unwrap()));
        assert!(!whitelist.is_allowed(&"8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn whitelist_ignores_invalid_entries() {
        let whitelist = IpWhitelist::new(&[
            "192.168.1.1".to_string(),
            "invalid".to_string(),
            "10.0.0.0/8".to_string(),
        ]);
        assert!(whitelist.is_enabled());
        // Valid entries still work
        assert!(whitelist.is_allowed(&"192.168.1.1".parse().unwrap()));
        assert!(whitelist.is_allowed(&"10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn localhost_variations() {
        let whitelist = IpWhitelist::new(&["127.0.0.1".to_string(), "::1".to_string()]);
        assert!(whitelist.is_allowed(&"127.0.0.1".parse().unwrap()));
        assert!(whitelist.is_allowed(&"::1".parse().unwrap()));
        assert!(!whitelist.is_allowed(&"127.0.0.2".parse().unwrap()));
    }
}
