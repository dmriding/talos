//! API Token management for Talos service authentication.
//!
//! This module provides database-backed token management for API authentication.
//! Tokens are stored as SHA-256 hashes in the database, and the raw token is only
//! returned once at creation time.
//!
//! # Usage
//!
//! ```rust,ignore
//! use talos::server::tokens::{ApiToken, TokenManager};
//!
//! // Create a new token
//! let (token, raw) = manager.create_token("My Service", &["licenses:read"]).await?;
//! println!("Save this token: {}", raw); // Only shown once!
//!
//! // Validate a token from a request
//! let token = manager.validate_token(&raw_token).await?;
//! if token.has_scope("licenses:read") {
//!     // Allow the request
//! }
//! ```

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{query, FromRow};
use tracing::{info, warn};
use uuid::Uuid;

use crate::errors::{LicenseError, LicenseResult};
use crate::server::database::Database;
use crate::server::handlers::AppState;

/// API Token stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiToken {
    /// Unique identifier for the token
    pub id: String,
    /// Human-readable name for the token
    pub name: String,
    /// SHA-256 hash of the token (never store raw tokens)
    #[serde(skip_serializing)]
    pub token_hash: String,
    /// Space-separated list of scopes
    pub scopes: String,
    /// When the token was created
    pub created_at: NaiveDateTime,
    /// When the token expires (None = never)
    pub expires_at: Option<NaiveDateTime>,
    /// Last time the token was used
    pub last_used_at: Option<NaiveDateTime>,
    /// When the token was revoked (None = active)
    pub revoked_at: Option<NaiveDateTime>,
    /// Who created this token
    pub created_by: Option<String>,
}

impl ApiToken {
    /// Check if the token has a specific scope.
    pub fn has_scope(&self, required: &str) -> bool {
        // Check for wildcard scope
        if self.scopes.split_whitespace().any(|s| s == "*") {
            return true;
        }

        // Check for exact match or category wildcard
        for scope in self.scopes.split_whitespace() {
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

    /// Check if the token is valid (not expired, not revoked).
    pub fn is_valid(&self) -> bool {
        // Check if revoked
        if self.revoked_at.is_some() {
            return false;
        }

        // Check if expired
        if let Some(expires_at) = self.expires_at {
            if Utc::now().naive_utc() > expires_at {
                return false;
            }
        }

        true
    }

    /// Get the scopes as a vector.
    pub fn scope_list(&self) -> Vec<String> {
        self.scopes.split_whitespace().map(String::from).collect()
    }
}

/// Response for token creation (includes the raw token).
#[derive(Debug, Serialize)]
pub struct CreateTokenResponse {
    /// The created token metadata
    pub token: TokenMetadata,
    /// The raw token value - ONLY RETURNED ONCE
    pub raw_token: String,
}

/// Token metadata for listing (excludes hash and raw token).
#[derive(Debug, Clone, Serialize)]
pub struct TokenMetadata {
    pub id: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
    pub revoked_at: Option<String>,
    pub created_by: Option<String>,
    pub is_active: bool,
}

impl From<ApiToken> for TokenMetadata {
    fn from(token: ApiToken) -> Self {
        let is_active = token.is_valid();
        let scopes = token.scope_list();
        TokenMetadata {
            id: token.id,
            name: token.name,
            scopes,
            created_at: token.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            expires_at: token.expires_at.map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            last_used_at: token.last_used_at.map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            revoked_at: token.revoked_at.map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            created_by: token.created_by,
            is_active,
        }
    }
}

/// Request to create a new API token.
#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    /// Human-readable name for the token
    pub name: String,
    /// Space-separated or array of scopes
    pub scopes: Vec<String>,
    /// Optional expiration (ISO 8601 format)
    pub expires_at: Option<String>,
}

/// Generate a secure random token.
fn generate_raw_token() -> String {
    // Generate a UUID-based token with prefix for easy identification
    format!("talos_{}", Uuid::new_v4().to_string().replace('-', ""))
}

/// Hash a token using SHA-256.
fn hash_token(raw_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    format!("{:x}", hasher.finalize())
}

impl Database {
    /// Create a new API token.
    ///
    /// Returns the token metadata and the raw token value (only returned once).
    pub async fn create_api_token(
        &self,
        name: &str,
        scopes: &[&str],
        expires_at: Option<NaiveDateTime>,
        created_by: Option<&str>,
    ) -> LicenseResult<(ApiToken, String)> {
        let id = Uuid::new_v4().to_string();
        let raw_token = generate_raw_token();
        let token_hash = hash_token(&raw_token);
        let now = Utc::now().naive_utc();
        let scopes_str = scopes.join(" ");

        let token = ApiToken {
            id: id.clone(),
            name: name.to_string(),
            token_hash: token_hash.clone(),
            scopes: scopes_str.clone(),
            created_at: now,
            expires_at,
            last_used_at: None,
            revoked_at: None,
            created_by: created_by.map(String::from),
        };

        match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                query(
                    "INSERT INTO api_tokens (id, name, token_hash, scopes, created_at, expires_at, created_by) \
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(name)
                .bind(&token_hash)
                .bind(&scopes_str)
                .bind(now)
                .bind(expires_at)
                .bind(created_by)
                .execute(pool)
                .await
                .map_err(|e| LicenseError::ServerError(format!("failed to create token: {e}")))?;
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                query(
                    "INSERT INTO api_tokens (id, name, token_hash, scopes, created_at, expires_at, created_by) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7)",
                )
                .bind(&id)
                .bind(name)
                .bind(&token_hash)
                .bind(&scopes_str)
                .bind(now)
                .bind(expires_at)
                .bind(created_by)
                .execute(pool)
                .await
                .map_err(|e| LicenseError::ServerError(format!("failed to create token: {e}")))?;
            }
        }

        info!("Created API token '{}' with id={}", name, id);
        Ok((token, raw_token))
    }

    /// Validate a raw token and return the token if valid.
    ///
    /// Also updates `last_used_at` timestamp.
    pub async fn validate_api_token(&self, raw_token: &str) -> LicenseResult<Option<ApiToken>> {
        let token_hash = hash_token(raw_token);

        let token: Option<ApiToken> = match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                sqlx::query_as::<_, ApiToken>(
                    "SELECT id, name, token_hash, scopes, created_at, expires_at, \
                            last_used_at, revoked_at, created_by \
                     FROM api_tokens WHERE token_hash = ?",
                )
                .bind(&token_hash)
                .fetch_optional(pool)
                .await
                .map_err(|e| LicenseError::ServerError(format!("token lookup failed: {e}")))?
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                sqlx::query_as::<_, ApiToken>(
                    "SELECT id, name, token_hash, scopes, created_at, expires_at, \
                            last_used_at, revoked_at, created_by \
                     FROM api_tokens WHERE token_hash = $1",
                )
                .bind(&token_hash)
                .fetch_optional(pool)
                .await
                .map_err(|e| LicenseError::ServerError(format!("token lookup failed: {e}")))?
            }
        };

        if let Some(ref t) = token {
            if t.is_valid() {
                // Update last_used_at
                self.update_token_last_used(&t.id).await?;
            }
        }

        Ok(token)
    }

    /// Update the last_used_at timestamp for a token.
    async fn update_token_last_used(&self, token_id: &str) -> LicenseResult<()> {
        let now = Utc::now().naive_utc();

        match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                query("UPDATE api_tokens SET last_used_at = ? WHERE id = ?")
                    .bind(now)
                    .bind(token_id)
                    .execute(pool)
                    .await
                    .map_err(|e| LicenseError::ServerError(format!("update last_used failed: {e}")))?;
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                query("UPDATE api_tokens SET last_used_at = $1 WHERE id = $2")
                    .bind(now)
                    .bind(token_id)
                    .execute(pool)
                    .await
                    .map_err(|e| LicenseError::ServerError(format!("update last_used failed: {e}")))?;
            }
        }

        Ok(())
    }

    /// List all API tokens (metadata only, no hashes).
    pub async fn list_api_tokens(&self) -> LicenseResult<Vec<ApiToken>> {
        match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                sqlx::query_as::<_, ApiToken>(
                    "SELECT id, name, token_hash, scopes, created_at, expires_at, \
                            last_used_at, revoked_at, created_by \
                     FROM api_tokens ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(|e| LicenseError::ServerError(format!("list tokens failed: {e}")))
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                sqlx::query_as::<_, ApiToken>(
                    "SELECT id, name, token_hash, scopes, created_at, expires_at, \
                            last_used_at, revoked_at, created_by \
                     FROM api_tokens ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(|e| LicenseError::ServerError(format!("list tokens failed: {e}")))
            }
        }
    }

    /// Get a token by ID.
    pub async fn get_api_token(&self, token_id: &str) -> LicenseResult<Option<ApiToken>> {
        match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                sqlx::query_as::<_, ApiToken>(
                    "SELECT id, name, token_hash, scopes, created_at, expires_at, \
                            last_used_at, revoked_at, created_by \
                     FROM api_tokens WHERE id = ?",
                )
                .bind(token_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| LicenseError::ServerError(format!("get token failed: {e}")))
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                sqlx::query_as::<_, ApiToken>(
                    "SELECT id, name, token_hash, scopes, created_at, expires_at, \
                            last_used_at, revoked_at, created_by \
                     FROM api_tokens WHERE id = $1",
                )
                .bind(token_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| LicenseError::ServerError(format!("get token failed: {e}")))
            }
        }
    }

    /// Revoke a token by ID.
    pub async fn revoke_api_token(&self, token_id: &str) -> LicenseResult<bool> {
        let now = Utc::now().naive_utc();

        let rows_affected = match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                query("UPDATE api_tokens SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL")
                    .bind(now)
                    .bind(token_id)
                    .execute(pool)
                    .await
                    .map_err(|e| LicenseError::ServerError(format!("revoke token failed: {e}")))?
                    .rows_affected()
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                query("UPDATE api_tokens SET revoked_at = $1 WHERE id = $2 AND revoked_at IS NULL")
                    .bind(now)
                    .bind(token_id)
                    .execute(pool)
                    .await
                    .map_err(|e| LicenseError::ServerError(format!("revoke token failed: {e}")))?
                    .rows_affected()
            }
        };

        if rows_affected > 0 {
            warn!("Revoked API token id={}", token_id);
        }

        Ok(rows_affected > 0)
    }

    /// Check if any API tokens exist in the database.
    pub async fn has_api_tokens(&self) -> LicenseResult<bool> {
        match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_tokens")
                    .fetch_one(pool)
                    .await
                    .map_err(|e| LicenseError::ServerError(format!("count tokens failed: {e}")))?;
                Ok(count.0 > 0)
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_tokens")
                    .fetch_one(pool)
                    .await
                    .map_err(|e| LicenseError::ServerError(format!("count tokens failed: {e}")))?;
                Ok(count.0 > 0)
            }
        }
    }
}

// ============================================================================
// HTTP Handlers for Token Management
// ============================================================================

/// Response for token list endpoint.
#[derive(Debug, Serialize)]
pub struct ListTokensResponse {
    pub tokens: Vec<TokenMetadata>,
}

/// Response for single token operations.
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub token: TokenMetadata,
}

/// Response for token revocation.
#[derive(Debug, Serialize)]
pub struct RevokeTokenResponse {
    pub success: bool,
    pub message: String,
}

/// Error response for token operations.
#[derive(Debug, Serialize)]
pub struct TokenErrorResponse {
    pub error: String,
    pub code: String,
}

impl TokenErrorResponse {
    fn new(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
        }
    }
}

/// POST /api/v1/tokens - Create a new API token.
///
/// Request body: `CreateTokenRequest`
/// Response: `CreateTokenResponse` (includes raw token, only shown once)
pub async fn create_token_handler(
    State(state): State<AppState>,
    Json(req): Json<CreateTokenRequest>,
) -> impl IntoResponse {
    // Validate request
    if req.name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!(TokenErrorResponse::new(
                "Token name is required",
                "INVALID_NAME"
            ))),
        )
            .into_response();
    }

    if req.scopes.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!(TokenErrorResponse::new(
                "At least one scope is required",
                "INVALID_SCOPES"
            ))),
        )
            .into_response();
    }

    // Parse optional expiration
    let expires_at = match &req.expires_at {
        Some(exp_str) => match NaiveDateTime::parse_from_str(exp_str, "%Y-%m-%dT%H:%M:%SZ") {
            Ok(dt) => Some(dt),
            Err(_) => match NaiveDateTime::parse_from_str(exp_str, "%Y-%m-%dT%H:%M:%S") {
                Ok(dt) => Some(dt),
                Err(_) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!(TokenErrorResponse::new(
                            "Invalid expires_at format. Use ISO 8601 format.",
                            "INVALID_EXPIRATION"
                        ))),
                    )
                        .into_response();
                }
            },
        },
        None => None,
    };

    // Convert scopes to &str slice
    let scope_refs: Vec<&str> = req.scopes.iter().map(|s| s.as_str()).collect();

    // Create the token
    match state
        .db
        .create_api_token(&req.name, &scope_refs, expires_at, None)
        .await
    {
        Ok((token, raw_token)) => {
            let response = CreateTokenResponse {
                token: TokenMetadata::from(token),
                raw_token,
            };
            (StatusCode::CREATED, Json(serde_json::json!(response))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(TokenErrorResponse::new(
                format!("Failed to create token: {}", e),
                "CREATE_FAILED"
            ))),
        )
            .into_response(),
    }
}

/// GET /api/v1/tokens - List all API tokens.
///
/// Response: `ListTokensResponse`
pub async fn list_tokens_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.list_api_tokens().await {
        Ok(tokens) => {
            let metadata: Vec<TokenMetadata> = tokens.into_iter().map(TokenMetadata::from).collect();
            let response = ListTokensResponse { tokens: metadata };
            (StatusCode::OK, Json(serde_json::json!(response))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(TokenErrorResponse::new(
                format!("Failed to list tokens: {}", e),
                "LIST_FAILED"
            ))),
        )
            .into_response(),
    }
}

/// GET /api/v1/tokens/:id - Get a specific token by ID.
///
/// Response: `TokenResponse`
pub async fn get_token_handler(
    State(state): State<AppState>,
    Path(token_id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_api_token(&token_id).await {
        Ok(Some(token)) => {
            let response = TokenResponse {
                token: TokenMetadata::from(token),
            };
            (StatusCode::OK, Json(serde_json::json!(response))).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!(TokenErrorResponse::new(
                "Token not found",
                "NOT_FOUND"
            ))),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(TokenErrorResponse::new(
                format!("Failed to get token: {}", e),
                "GET_FAILED"
            ))),
        )
            .into_response(),
    }
}

/// DELETE /api/v1/tokens/:id - Revoke a token.
///
/// Response: `RevokeTokenResponse`
pub async fn revoke_token_handler(
    State(state): State<AppState>,
    Path(token_id): Path<String>,
) -> impl IntoResponse {
    match state.db.revoke_api_token(&token_id).await {
        Ok(true) => {
            let response = RevokeTokenResponse {
                success: true,
                message: "Token revoked successfully".to_string(),
            };
            (StatusCode::OK, Json(serde_json::json!(response))).into_response()
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!(TokenErrorResponse::new(
                "Token not found or already revoked",
                "NOT_FOUND"
            ))),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(TokenErrorResponse::new(
                format!("Failed to revoke token: {}", e),
                "REVOKE_FAILED"
            ))),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_has_scope_exact_match() {
        let token = ApiToken {
            id: "test".to_string(),
            name: "Test".to_string(),
            token_hash: "hash".to_string(),
            scopes: "licenses:read licenses:write".to_string(),
            created_at: Utc::now().naive_utc(),
            expires_at: None,
            last_used_at: None,
            revoked_at: None,
            created_by: None,
        };

        assert!(token.has_scope("licenses:read"));
        assert!(token.has_scope("licenses:write"));
        assert!(!token.has_scope("licenses:delete"));
        assert!(!token.has_scope("admin:read"));
    }

    #[test]
    fn token_has_scope_wildcard() {
        let token = ApiToken {
            id: "test".to_string(),
            name: "Test".to_string(),
            token_hash: "hash".to_string(),
            scopes: "*".to_string(),
            created_at: Utc::now().naive_utc(),
            expires_at: None,
            last_used_at: None,
            revoked_at: None,
            created_by: None,
        };

        assert!(token.has_scope("licenses:read"));
        assert!(token.has_scope("anything:here"));
    }

    #[test]
    fn token_has_scope_category_wildcard() {
        let token = ApiToken {
            id: "test".to_string(),
            name: "Test".to_string(),
            token_hash: "hash".to_string(),
            scopes: "licenses:*".to_string(),
            created_at: Utc::now().naive_utc(),
            expires_at: None,
            last_used_at: None,
            revoked_at: None,
            created_by: None,
        };

        assert!(token.has_scope("licenses:read"));
        assert!(token.has_scope("licenses:write"));
        assert!(token.has_scope("licenses:delete"));
        assert!(!token.has_scope("admin:read"));
    }

    #[test]
    fn token_is_valid_active() {
        let token = ApiToken {
            id: "test".to_string(),
            name: "Test".to_string(),
            token_hash: "hash".to_string(),
            scopes: "*".to_string(),
            created_at: Utc::now().naive_utc(),
            expires_at: None,
            last_used_at: None,
            revoked_at: None,
            created_by: None,
        };

        assert!(token.is_valid());
    }

    #[test]
    fn token_is_valid_revoked() {
        let token = ApiToken {
            id: "test".to_string(),
            name: "Test".to_string(),
            token_hash: "hash".to_string(),
            scopes: "*".to_string(),
            created_at: Utc::now().naive_utc(),
            expires_at: None,
            last_used_at: None,
            revoked_at: Some(Utc::now().naive_utc()),
            created_by: None,
        };

        assert!(!token.is_valid());
    }

    #[test]
    fn token_is_valid_expired() {
        let token = ApiToken {
            id: "test".to_string(),
            name: "Test".to_string(),
            token_hash: "hash".to_string(),
            scopes: "*".to_string(),
            created_at: Utc::now().naive_utc(),
            expires_at: Some(Utc::now().naive_utc() - chrono::Duration::hours(1)),
            last_used_at: None,
            revoked_at: None,
            created_by: None,
        };

        assert!(!token.is_valid());
    }

    #[test]
    fn hash_token_produces_sha256() {
        let raw = "talos_abc123";
        let hash = hash_token(raw);
        // SHA-256 produces 64 hex characters
        assert_eq!(hash.len(), 64);
        // Same input should produce same hash
        assert_eq!(hash, hash_token(raw));
        // Different input should produce different hash
        assert_ne!(hash, hash_token("talos_xyz789"));
    }

    #[test]
    fn generate_raw_token_format() {
        let token = generate_raw_token();
        assert!(token.starts_with("talos_"));
        // UUID without dashes = 32 chars, plus "talos_" = 38 chars
        assert_eq!(token.len(), 38);
    }
}
