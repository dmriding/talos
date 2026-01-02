//! JWT authentication middleware for Talos admin API.
//!
//! This module provides JWT-based authentication for admin endpoints.
//! It is only available when the `jwt-auth` feature is enabled.
//!
//! # Usage
//!
//! ```rust,ignore
//! use talos::server::auth::{AuthenticatedUser, JwtLayer};
//!
//! // Create auth layer for protected routes
//! let auth_layer = JwtLayer::from_config(&config.auth)?;
//!
//! // Use in route handler via extractor
//! async fn admin_handler(user: AuthenticatedUser) -> impl IntoResponse {
//!     format!("Hello, {}!", user.subject)
//! }
//! ```
//!
//! # Scopes
//!
//! JWT tokens can include scopes to control access:
//! - `licenses:read` - Read license information
//! - `licenses:write` - Create and modify licenses
//! - `licenses:*` - Full license access
//!
//! # Configuration
//!
//! Set via environment variables or config.toml:
//! - `TALOS_JWT_SECRET` - Required secret key for HS256 signing
//! - `TALOS_JWT_ISSUER` - Expected issuer claim (default: "talos")
//! - `TALOS_JWT_AUDIENCE` - Expected audience claim (default: "talos-api")

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::AuthConfig;
use crate::errors::{LicenseError, LicenseResult};

/// JWT claims structure.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (typically user ID or service name)
    pub sub: String,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// Expiration time (Unix timestamp)
    pub exp: u64,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// Scopes (space-separated list)
    #[serde(default)]
    pub scope: String,
}

impl Claims {
    /// Check if the claims include a specific scope.
    pub fn has_scope(&self, required: &str) -> bool {
        // Check for wildcard scope
        if self.scope.split_whitespace().any(|s| s == "*") {
            return true;
        }

        // Check for exact match or category wildcard (e.g., "licenses:*" matches "licenses:read")
        for scope in self.scope.split_whitespace() {
            if scope == required {
                return true;
            }
            // Check for wildcard match: "licenses:*" matches "licenses:read"
            if let Some(prefix) = scope.strip_suffix(":*") {
                if required.starts_with(prefix) && required.chars().nth(prefix.len()) == Some(':') {
                    return true;
                }
            }
        }

        false
    }
}

/// Authenticated user information extracted from JWT.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// The subject from the JWT (user ID or service name)
    pub subject: String,
    /// Scopes from the JWT
    pub scopes: Vec<String>,
    /// Full claims for advanced use cases
    pub claims: Claims,
}

impl AuthenticatedUser {
    /// Check if the user has a specific scope.
    pub fn has_scope(&self, scope: &str) -> bool {
        self.claims.has_scope(scope)
    }

    /// Require a specific scope, returning an error if not present.
    pub fn require_scope(&self, scope: &str) -> Result<(), AuthError> {
        if self.has_scope(scope) {
            Ok(())
        } else {
            Err(AuthError::InsufficientScope(scope.to_string()))
        }
    }
}

/// Authentication errors.
#[derive(Debug, Clone)]
pub enum AuthError {
    /// Missing Authorization header
    MissingToken,
    /// Invalid Authorization header format
    InvalidHeader,
    /// Token validation failed
    InvalidToken(String),
    /// Token has expired
    TokenExpired,
    /// Insufficient scope for the requested operation
    InsufficientScope(String),
    /// Auth is not configured/enabled
    AuthDisabled,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingToken => write!(f, "missing authorization token"),
            AuthError::InvalidHeader => write!(f, "invalid authorization header format"),
            AuthError::InvalidToken(msg) => write!(f, "invalid token: {msg}"),
            AuthError::TokenExpired => write!(f, "token has expired"),
            AuthError::InsufficientScope(scope) => {
                write!(f, "insufficient scope: requires {scope}")
            }
            AuthError::AuthDisabled => write!(f, "authentication is not enabled"),
        }
    }
}

impl std::error::Error for AuthError {}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::InvalidHeader => (StatusCode::BAD_REQUEST, self.to_string()),
            AuthError::InvalidToken(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::InsufficientScope(_) => (StatusCode::FORBIDDEN, self.to_string()),
            AuthError::AuthDisabled => (StatusCode::NOT_IMPLEMENTED, self.to_string()),
        };

        let body = serde_json::json!({
            "error": message,
            "code": match &self {
                AuthError::MissingToken => "MISSING_TOKEN",
                AuthError::InvalidHeader => "INVALID_HEADER",
                AuthError::InvalidToken(_) => "INVALID_TOKEN",
                AuthError::TokenExpired => "TOKEN_EXPIRED",
                AuthError::InsufficientScope(_) => "INSUFFICIENT_SCOPE",
                AuthError::AuthDisabled => "AUTH_DISABLED",
            }
        });

        (status, Json(body)).into_response()
    }
}

/// JWT validator for token verification.
#[derive(Clone)]
pub struct JwtValidator {
    decoding_key: DecodingKey,
    encoding_key: EncodingKey,
    validation: Validation,
    issuer: String,
    audience: String,
    expiration_secs: u64,
}

impl JwtValidator {
    /// Create a new JWT validator from auth configuration.
    pub fn from_config(config: &AuthConfig) -> LicenseResult<Self> {
        if config.jwt_secret.is_empty() {
            return Err(LicenseError::ConfigError(
                "jwt_secret is required for JWT authentication".to_string(),
            ));
        }

        // Resolve secret (support env: prefix for environment variable)
        let secret = if let Some(env_var) = config.jwt_secret.strip_prefix("env:") {
            std::env::var(env_var).map_err(|_| {
                LicenseError::ConfigError(format!(
                    "environment variable '{env_var}' not found for jwt_secret"
                ))
            })?
        } else {
            config.jwt_secret.clone()
        };

        let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.set_issuer(&[&config.jwt_issuer]);
        validation.set_audience(&[&config.jwt_audience]);
        validation.validate_exp = true;

        Ok(Self {
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            validation,
            issuer: config.jwt_issuer.clone(),
            audience: config.jwt_audience.clone(),
            expiration_secs: config.token_expiration_secs,
        })
    }

    /// Validate a JWT token and extract claims.
    pub fn validate_token(&self, token: &str) -> Result<TokenData<Claims>, AuthError> {
        decode::<Claims>(token, &self.decoding_key, &self.validation).map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            _ => AuthError::InvalidToken(e.to_string()),
        })
    }

    /// Create a new JWT token with the given subject and scopes.
    pub fn create_token(&self, subject: &str, scopes: &[&str]) -> LicenseResult<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| LicenseError::ServerError(format!("system time error: {e}")))?
            .as_secs();

        let claims = Claims {
            sub: subject.to_string(),
            iat: now,
            exp: now + self.expiration_secs,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            scope: scopes.join(" "),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| LicenseError::ServerError(format!("failed to create token: {e}")))
    }
}

impl std::fmt::Debug for JwtValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtValidator")
            .field("issuer", &self.issuer)
            .field("audience", &self.audience)
            .field("expiration_secs", &self.expiration_secs)
            .finish()
    }
}

/// State extension for JWT authentication.
///
/// Add this to your AppState to enable JWT authentication in handlers.
#[derive(Clone)]
pub struct AuthState {
    /// Whether auth is enabled
    pub enabled: bool,
    /// JWT validator (None if auth is disabled)
    pub validator: Option<Arc<JwtValidator>>,
}

impl AuthState {
    /// Create auth state from configuration.
    pub fn from_config(config: &AuthConfig) -> LicenseResult<Self> {
        if !config.enabled {
            return Ok(Self {
                enabled: false,
                validator: None,
            });
        }

        let validator = JwtValidator::from_config(config)?;
        Ok(Self {
            enabled: true,
            validator: Some(Arc::new(validator)),
        })
    }

    /// Create a disabled auth state.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            validator: None,
        }
    }
}

impl std::fmt::Debug for AuthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthState")
            .field("enabled", &self.enabled)
            .finish()
    }
}

/// Axum extractor for authenticated requests.
///
/// Use this in your handler signature to require authentication:
///
/// ```rust,ignore
/// async fn protected_handler(
///     user: AuthenticatedUser,
/// ) -> impl IntoResponse {
///     format!("Hello, {}!", user.subject)
/// }
/// ```
#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    AuthState: FromRequestParts<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get auth state from app state
        let auth_state = parts
            .extensions
            .get::<AuthState>()
            .cloned()
            .ok_or(AuthError::AuthDisabled)?;

        if !auth_state.enabled {
            return Err(AuthError::AuthDisabled);
        }

        let validator = auth_state
            .validator
            .as_ref()
            .ok_or(AuthError::AuthDisabled)?;

        // Extract Authorization header
        let auth_header = parts
            .headers
            .get("Authorization")
            .ok_or(AuthError::MissingToken)?
            .to_str()
            .map_err(|_| AuthError::InvalidHeader)?;

        // Parse Bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidHeader)?;

        // Validate token
        let token_data = validator.validate_token(token)?;
        let claims = token_data.claims;

        Ok(AuthenticatedUser {
            subject: claims.sub.clone(),
            scopes: claims.scope.split_whitespace().map(String::from).collect(),
            claims,
        })
    }
}

/// Optional authenticated user extractor.
///
/// Returns `Some(AuthenticatedUser)` if authentication succeeds, `None` otherwise.
/// Useful for endpoints that behave differently based on auth status.
#[derive(Debug, Clone)]
pub struct OptionalUser(pub Option<AuthenticatedUser>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalUser
where
    S: Send + Sync,
    AuthState: FromRequestParts<S>,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        match AuthenticatedUser::from_request_parts(parts, _state).await {
            Ok(user) => Ok(OptionalUser(Some(user))),
            Err(_) => Ok(OptionalUser(None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> AuthConfig {
        AuthConfig {
            enabled: true,
            jwt_secret: "test-secret-key-for-testing-only".to_string(),
            jwt_issuer: "talos".to_string(),
            jwt_audience: "talos-api".to_string(),
            token_expiration_secs: 3600,
        }
    }

    #[test]
    fn create_and_validate_token() {
        let config = test_config();
        let validator = JwtValidator::from_config(&config).unwrap();

        let token = validator
            .create_token("test-user", &["licenses:read", "licenses:write"])
            .unwrap();

        let token_data = validator.validate_token(&token).unwrap();
        assert_eq!(token_data.claims.sub, "test-user");
        assert!(token_data.claims.scope.contains("licenses:read"));
        assert!(token_data.claims.scope.contains("licenses:write"));
    }

    #[test]
    fn reject_invalid_token() {
        let config = test_config();
        let validator = JwtValidator::from_config(&config).unwrap();

        let result = validator.validate_token("invalid-token");
        assert!(result.is_err());
    }

    #[test]
    fn reject_wrong_secret() {
        let config = test_config();
        let validator = JwtValidator::from_config(&config).unwrap();

        let token = validator
            .create_token("test-user", &["licenses:read"])
            .unwrap();

        // Create a different validator with different secret
        let other_config = AuthConfig {
            jwt_secret: "different-secret".to_string(),
            ..test_config()
        };
        let other_validator = JwtValidator::from_config(&other_config).unwrap();

        let result = other_validator.validate_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn scope_matching() {
        let claims = Claims {
            sub: "test".to_string(),
            iat: 0,
            exp: u64::MAX,
            iss: "talos".to_string(),
            aud: "talos-api".to_string(),
            scope: "licenses:read licenses:write".to_string(),
        };

        assert!(claims.has_scope("licenses:read"));
        assert!(claims.has_scope("licenses:write"));
        assert!(!claims.has_scope("licenses:delete"));
        assert!(!claims.has_scope("admin:*"));
    }

    #[test]
    fn wildcard_scope_matching() {
        let claims = Claims {
            sub: "test".to_string(),
            iat: 0,
            exp: u64::MAX,
            iss: "talos".to_string(),
            aud: "talos-api".to_string(),
            scope: "licenses:*".to_string(),
        };

        assert!(claims.has_scope("licenses:read"));
        assert!(claims.has_scope("licenses:write"));
        assert!(claims.has_scope("licenses:delete"));
        assert!(!claims.has_scope("admin:read"));
    }

    #[test]
    fn global_wildcard_scope() {
        let claims = Claims {
            sub: "test".to_string(),
            iat: 0,
            exp: u64::MAX,
            iss: "talos".to_string(),
            aud: "talos-api".to_string(),
            scope: "*".to_string(),
        };

        assert!(claims.has_scope("licenses:read"));
        assert!(claims.has_scope("admin:anything"));
        assert!(claims.has_scope("any:scope:here"));
    }

    #[test]
    fn empty_secret_fails() {
        let config = AuthConfig {
            enabled: true,
            jwt_secret: "".to_string(),
            ..Default::default()
        };

        let result = JwtValidator::from_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn disabled_auth_state() {
        let config = AuthConfig {
            enabled: false,
            ..Default::default()
        };

        let state = AuthState::from_config(&config).unwrap();
        assert!(!state.enabled);
        assert!(state.validator.is_none());
    }

    #[test]
    fn enabled_auth_state() {
        let config = test_config();
        let state = AuthState::from_config(&config).unwrap();
        assert!(state.enabled);
        assert!(state.validator.is_some());
    }

    #[test]
    fn token_contains_correct_claims() {
        let config = test_config();
        let validator = JwtValidator::from_config(&config).unwrap();

        let token = validator
            .create_token("service-account", &["licenses:*"])
            .unwrap();
        let token_data = validator.validate_token(&token).unwrap();

        assert_eq!(token_data.claims.sub, "service-account");
        assert_eq!(token_data.claims.iss, "talos");
        assert_eq!(token_data.claims.aud, "talos-api");
        assert_eq!(token_data.claims.scope, "licenses:*");
        assert!(token_data.claims.exp > token_data.claims.iat);
    }

    #[test]
    fn authenticated_user_scope_check() {
        let claims = Claims {
            sub: "user".to_string(),
            iat: 0,
            exp: u64::MAX,
            iss: "talos".to_string(),
            aud: "talos-api".to_string(),
            scope: "licenses:read".to_string(),
        };

        let user = AuthenticatedUser {
            subject: claims.sub.clone(),
            scopes: claims.scope.split_whitespace().map(String::from).collect(),
            claims,
        };

        assert!(user.has_scope("licenses:read"));
        assert!(!user.has_scope("licenses:write"));
        assert!(user.require_scope("licenses:read").is_ok());
        assert!(user.require_scope("licenses:write").is_err());
    }

    #[test]
    fn reject_expired_token() {
        let config = test_config();
        let validator = JwtValidator::from_config(&config).unwrap();

        // Manually create a token with an expired timestamp (1 hour in the past)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let expired_claims = Claims {
            sub: "test-user".to_string(),
            iat: now - 7200, // 2 hours ago
            exp: now - 3600, // 1 hour ago (expired)
            iss: config.jwt_issuer.clone(),
            aud: config.jwt_audience.clone(),
            scope: "licenses:read".to_string(),
        };

        let token = encode(
            &Header::default(),
            &expired_claims,
            &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
        )
        .unwrap();

        // Token should be rejected as expired
        let result = validator.validate_token(&token);
        assert!(matches!(result, Err(AuthError::TokenExpired)));
    }

    #[test]
    fn reject_wrong_issuer() {
        let config = test_config();
        let validator = JwtValidator::from_config(&config).unwrap();

        let token = validator
            .create_token("test-user", &["licenses:read"])
            .unwrap();

        // Create validator expecting different issuer
        let other_config = AuthConfig {
            jwt_issuer: "other-issuer".to_string(),
            ..test_config()
        };
        let other_validator = JwtValidator::from_config(&other_config).unwrap();

        let result = other_validator.validate_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn reject_wrong_audience() {
        let config = test_config();
        let validator = JwtValidator::from_config(&config).unwrap();

        let token = validator
            .create_token("test-user", &["licenses:read"])
            .unwrap();

        // Create validator expecting different audience
        let other_config = AuthConfig {
            jwt_audience: "other-audience".to_string(),
            ..test_config()
        };
        let other_validator = JwtValidator::from_config(&other_config).unwrap();

        let result = other_validator.validate_token(&token);
        assert!(result.is_err());
    }
}
