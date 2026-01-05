//! Configuration system for Talos.
//!
//! Configuration is loaded from multiple sources with the following precedence:
//! 1. Environment variables (highest priority)
//! 2. `config.toml` file
//! 3. Default values (lowest priority)
//!
//! # Environment Variables
//!
//! All configuration options can be overridden via environment variables:
//! - `TALOS_SERVER_HOST` - Server bind address
//! - `TALOS_SERVER_PORT` - Server port
//! - `TALOS_DATABASE_URL` - Database connection URL
//! - `TALOS_LICENSE_KEY_PREFIX` - License key prefix
//! - `TALOS_LOG_LEVEL` - Log level (trace, debug, info, warn, error)
//! - `TALOS_AUTH_ENABLED` - Enable JWT authentication (requires `jwt-auth` feature)
//! - `TALOS_JWT_SECRET` - JWT secret key for signing/validation
//! - `TALOS_JWT_ISSUER` - JWT issuer claim
//! - `TALOS_JWT_AUDIENCE` - JWT audience claim
//! - `TALOS_TOKEN_EXPIRATION_SECS` - Token expiration time in seconds

use config::Config;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::sync::OnceLock;

use crate::errors::{LicenseError, LicenseResult};
use crate::tiers::TierConfig;

/// Global configuration singleton.
static CONFIG: OnceLock<TalosConfig> = OnceLock::new();

/// Root configuration structure.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TalosConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// License key configuration
    pub license: LicenseConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// JWT authentication configuration (requires "jwt-auth" feature)
    pub auth: AuthConfig,
    /// Rate limiting configuration (requires "rate-limiting" feature)
    pub rate_limit: RateLimitConfig,
    /// Admin API configuration
    pub admin: AdminConfig,
    /// Tier configurations (optional, keyed by tier name)
    pub tiers: HashMap<String, TierConfig>,
}

/// Server configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Host address to bind to
    pub host: String,
    /// Port to listen on
    pub port: u16,
    /// Heartbeat interval in seconds
    pub heartbeat_interval: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            heartbeat_interval: 60,
        }
    }
}

/// License key generation configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LicenseConfig {
    /// Prefix for generated license keys (e.g., "LIC" -> "LIC-XXXX-XXXX-XXXX")
    pub key_prefix: String,
    /// Number of segments in the license key
    pub key_segments: u8,
    /// Characters per segment
    pub key_segment_length: u8,
}

impl Default for LicenseConfig {
    fn default() -> Self {
        Self {
            key_prefix: "LIC".to_string(),
            key_segments: 4,
            key_segment_length: 4,
        }
    }
}

/// Database configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    /// Database type: "sqlite" or "postgres"
    pub db_type: String,
    /// SQLite connection URL
    pub sqlite_url: String,
    /// PostgreSQL connection URL
    pub postgres_url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            db_type: "sqlite".to_string(),
            sqlite_url: "sqlite://talos.db".to_string(),
            postgres_url: "postgres://localhost/talos".to_string(),
        }
    }
}

/// Logging configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Enable logging
    pub enabled: bool,
    /// Log level: trace, debug, info, warn, error
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            level: "info".to_string(),
        }
    }
}

/// JWT authentication configuration.
///
/// Used when the `jwt-auth` feature is enabled.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// Enable JWT authentication for admin endpoints
    pub enabled: bool,
    /// JWT secret key (use `env:VAR_NAME` to read from environment)
    pub jwt_secret: String,
    /// JWT issuer claim (iss)
    pub jwt_issuer: String,
    /// JWT audience claim (aud)
    pub jwt_audience: String,
    /// Token expiration time in seconds (default: 1 hour)
    pub token_expiration_secs: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            jwt_secret: String::new(),
            jwt_issuer: "talos".to_string(),
            jwt_audience: "talos-api".to_string(),
            token_expiration_secs: 3600,
        }
    }
}

/// Rate limiting configuration.
///
/// Used when the `rate-limiting` feature is enabled.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RateLimitConfig {
    /// Enable rate limiting for public endpoints
    pub enabled: bool,
    /// Requests per minute for /validate endpoint
    pub validate_rpm: u32,
    /// Requests per minute for /heartbeat endpoint
    pub heartbeat_rpm: u32,
    /// Requests per minute for /bind and /release endpoints
    pub bind_rpm: u32,
    /// Burst size (allows short bursts above the limit)
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            validate_rpm: 100,
            heartbeat_rpm: 60,
            bind_rpm: 10,
            burst_size: 5,
        }
    }
}

/// Admin API configuration.
///
/// Controls security settings for the admin API endpoints.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AdminConfig {
    /// IP whitelist for admin API access.
    ///
    /// When non-empty, only requests from these IPs/CIDRs can access admin endpoints.
    /// Supports individual IPs (e.g., "192.168.1.100") and CIDR notation (e.g., "10.0.0.0/8").
    ///
    /// Example: ["127.0.0.1", "10.0.0.0/8", "192.168.0.0/16"]
    ///
    /// When empty (default), IP whitelisting is disabled and all IPs are allowed
    /// (though JWT auth may still be required).
    pub ip_whitelist: Vec<String>,

    /// Enable audit logging for admin actions.
    ///
    /// When enabled, all admin API actions are logged with user/token ID,
    /// license state changes, and authentication failures.
    pub audit_logging: bool,
}

impl TalosConfig {
    /// Load configuration from file and environment.
    ///
    /// Configuration is loaded in this order (later sources override earlier):
    /// 1. Default values
    /// 2. `config.toml` file (optional)
    /// 3. Environment variables
    fn load() -> LicenseResult<Self> {
        let builder = Config::builder()
            // Start with defaults
            .set_default("server.host", "127.0.0.1")
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("server.port", 8080)
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("server.heartbeat_interval", 60)
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("license.key_prefix", "LIC")
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("license.key_segments", 4)
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("license.key_segment_length", 4)
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("database.db_type", "sqlite")
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("database.sqlite_url", "sqlite://talos.db")
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("database.postgres_url", "postgres://localhost/talos")
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("logging.enabled", false)
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("logging.level", "info")
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("auth.enabled", false)
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("auth.jwt_secret", "")
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("auth.jwt_issuer", "talos")
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("auth.jwt_audience", "talos-api")
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("auth.token_expiration_secs", 3600)
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("rate_limit.enabled", true)
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("rate_limit.validate_rpm", 100)
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("rate_limit.heartbeat_rpm", 60)
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("rate_limit.bind_rpm", 10)
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_default("rate_limit.burst_size", 5)
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            // Load from config.toml (optional)
            .add_source(config::File::with_name("config").required(false))
            // Override with environment variables
            .set_override_option("server.host", env::var("TALOS_SERVER_HOST").ok())
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_override_option(
                "server.port",
                env::var("TALOS_SERVER_PORT")
                    .ok()
                    .and_then(|v| v.parse::<i64>().ok()),
            )
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_override_option(
                "server.heartbeat_interval",
                env::var("TALOS_HEARTBEAT_INTERVAL")
                    .ok()
                    .and_then(|v| v.parse::<i64>().ok()),
            )
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_override_option(
                "license.key_prefix",
                env::var("TALOS_LICENSE_KEY_PREFIX").ok(),
            )
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_override_option("database.db_type", env::var("TALOS_DATABASE_TYPE").ok())
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_override_option(
                "database.sqlite_url",
                env::var("TALOS_DATABASE_URL")
                    .ok()
                    .filter(|url| url.starts_with("sqlite")),
            )
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_override_option(
                "database.postgres_url",
                env::var("TALOS_DATABASE_URL")
                    .ok()
                    .filter(|url| url.starts_with("postgres")),
            )
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_override_option(
                "logging.enabled",
                env::var("TALOS_LOGGING_ENABLED")
                    .ok()
                    .and_then(|v| v.parse::<bool>().ok()),
            )
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_override_option("logging.level", env::var("TALOS_LOG_LEVEL").ok())
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_override_option(
                "auth.enabled",
                env::var("TALOS_AUTH_ENABLED")
                    .ok()
                    .and_then(|v| v.parse::<bool>().ok()),
            )
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_override_option("auth.jwt_secret", env::var("TALOS_JWT_SECRET").ok())
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_override_option("auth.jwt_issuer", env::var("TALOS_JWT_ISSUER").ok())
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_override_option("auth.jwt_audience", env::var("TALOS_JWT_AUDIENCE").ok())
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?
            .set_override_option(
                "auth.token_expiration_secs",
                env::var("TALOS_TOKEN_EXPIRATION_SECS")
                    .ok()
                    .and_then(|v| v.parse::<i64>().ok()),
            )
            .map_err(|e| LicenseError::ConfigError(e.to_string()))?;

        let settings = builder
            .build()
            .map_err(|e| LicenseError::ConfigError(format!("failed to build config: {e}")))?;

        settings
            .try_deserialize()
            .map_err(|e| LicenseError::ConfigError(format!("failed to deserialize config: {e}")))
    }

    /// Validate the configuration.
    pub fn validate(&self) -> LicenseResult<()> {
        // Validate port
        if self.server.port == 0 {
            return Err(LicenseError::ConfigError(
                "server.port must be greater than 0".to_string(),
            ));
        }

        // Validate database type
        match self.database.db_type.as_str() {
            "sqlite" | "postgres" => {}
            other => {
                return Err(LicenseError::ConfigError(format!(
                    "database.db_type must be 'sqlite' or 'postgres', got '{other}'"
                )));
            }
        }

        // Validate license key config
        if self.license.key_prefix.is_empty() {
            return Err(LicenseError::ConfigError(
                "license.key_prefix cannot be empty".to_string(),
            ));
        }
        if self.license.key_segments == 0 {
            return Err(LicenseError::ConfigError(
                "license.key_segments must be greater than 0".to_string(),
            ));
        }
        if self.license.key_segment_length == 0 {
            return Err(LicenseError::ConfigError(
                "license.key_segment_length must be greater than 0".to_string(),
            ));
        }

        // Validate log level
        match self.logging.level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {}
            other => {
                return Err(LicenseError::ConfigError(format!(
                    "logging.level must be one of: trace, debug, info, warn, error. Got '{other}'"
                )));
            }
        }

        // Validate auth config (only if enabled)
        if self.auth.enabled && self.auth.jwt_secret.is_empty() {
            return Err(LicenseError::ConfigError(
                "auth.jwt_secret is required when auth.enabled is true".to_string(),
            ));
        }

        Ok(())
    }
}

/// Get the global configuration.
///
/// This loads the configuration on first access and caches it.
/// Returns an error if configuration loading or validation fails.
pub fn get_config() -> LicenseResult<&'static TalosConfig> {
    // Check if already initialized
    if let Some(config) = CONFIG.get() {
        return Ok(config);
    }

    // Load and validate configuration
    let config = TalosConfig::load()?;
    config.validate()?;

    // Try to set it (ignore if another thread beat us)
    let _ = CONFIG.set(config.clone());

    // Return the stored config (either ours or another thread's)
    Ok(CONFIG.get().expect("config was just set"))
}

/// Initialize configuration explicitly.
///
/// Call this early in your application to catch configuration errors.
/// Returns the validated configuration.
pub fn init_config() -> LicenseResult<&'static TalosConfig> {
    get_config()
}

// ============================================================================
// Legacy API (for backwards compatibility)
// ============================================================================

use crate::client::license::License;

/// Retrieve the server URL for Talos operations.
///
/// Precedence:
/// 1. `TALOS_SERVER_URL` or `SERVER_URL` environment variable
/// 2. `server.host` and `server.port` from config
/// 3. `license.server_url` as a final fallback
pub fn get_server_url(license: &License) -> String {
    // 1. Environment variable override (legacy support)
    if let Ok(url) = env::var("TALOS_SERVER_URL").or_else(|_| env::var("SERVER_URL")) {
        return url;
    }

    // 2. Try to get from config
    if let Ok(config) = get_config() {
        return format!("http://{}:{}", config.server.host, config.server.port);
    }

    // 3. Fallback to whatever is embedded in the license
    license.server_url.clone()
}

/// Retrieve the heartbeat interval in seconds.
pub fn get_heartbeat_interval() -> u64 {
    get_config()
        .map(|c| c.server.heartbeat_interval)
        .unwrap_or(60)
}

/// Check whether logging is enabled.
pub fn is_logging_enabled() -> bool {
    get_config().map(|c| c.logging.enabled).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> TalosConfig {
        TalosConfig::default()
    }

    #[test]
    fn default_config_is_valid() {
        let config = default_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validates_port_not_zero() {
        let mut config = default_config();
        config.server.port = 0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("port"));
    }

    #[test]
    fn validates_database_type() {
        let mut config = default_config();
        config.database.db_type = "invalid".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("db_type"));
    }

    #[test]
    fn validates_log_level() {
        let mut config = default_config();
        config.logging.level = "invalid".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("logging.level"));
    }

    #[test]
    fn validates_jwt_secret_required_when_auth_enabled() {
        let mut config = default_config();
        config.auth.enabled = true;
        config.auth.jwt_secret = String::new(); // Empty secret
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("jwt_secret"));
    }

    #[test]
    fn validates_jwt_secret_not_required_when_auth_disabled() {
        let mut config = default_config();
        config.auth.enabled = false;
        config.auth.jwt_secret = String::new(); // Empty secret is OK
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validates_license_key_prefix_not_empty() {
        let mut config = default_config();
        config.license.key_prefix = String::new();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("key_prefix"));
    }

    #[test]
    fn validates_license_key_segments_not_zero() {
        let mut config = default_config();
        config.license.key_segments = 0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("key_segments"));
    }

    #[test]
    fn validates_license_key_segment_length_not_zero() {
        let mut config = default_config();
        config.license.key_segment_length = 0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("key_segment_length"));
    }
}
