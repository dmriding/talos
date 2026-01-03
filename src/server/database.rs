use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, FromRow};
use std::sync::Arc;
use tracing::error;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "postgres")]
use sqlx::PgPool;

use crate::config::get_config;
use crate::errors::{LicenseError, LicenseResult};

/// Represents a license record stored in the database.
///
/// This mirrors the `licenses` table schema defined in your migrations.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct License {
    // === Core fields ===
    pub license_id: String,
    pub client_id: Option<String>,
    pub status: String,
    pub features: Option<String>,
    pub issued_at: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
    pub hardware_id: Option<String>,
    pub signature: Option<String>,
    pub last_heartbeat: Option<NaiveDateTime>,

    // === Organization fields ===
    pub org_id: Option<String>,
    pub org_name: Option<String>,

    // === License key (human-readable) ===
    pub license_key: Option<String>,

    // === Tier system ===
    pub tier: Option<String>,

    // === Extended hardware binding ===
    pub device_name: Option<String>,
    pub device_info: Option<String>,
    pub bound_at: Option<NaiveDateTime>,
    pub last_seen_at: Option<NaiveDateTime>,

    // === Status lifecycle ===
    pub suspended_at: Option<NaiveDateTime>,
    pub revoked_at: Option<NaiveDateTime>,
    pub revoke_reason: Option<String>,
    pub grace_period_ends_at: Option<NaiveDateTime>,
    pub suspension_message: Option<String>,

    // === Blacklist ===
    pub is_blacklisted: Option<bool>,
    pub blacklisted_at: Option<NaiveDateTime>,
    pub blacklist_reason: Option<String>,

    // === Metadata ===
    pub metadata: Option<String>,

    // === Quota/Usage tracking ===
    pub bandwidth_used_bytes: Option<i64>,
    pub bandwidth_limit_bytes: Option<i64>,
    pub quota_exceeded: Option<bool>,
}

impl License {
    /// Check if the license is currently bound to hardware.
    pub fn is_bound(&self) -> bool {
        self.hardware_id.is_some() && self.bound_at.is_some()
    }

    /// Check if the license is expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Utc::now().naive_utc()
        } else {
            false
        }
    }

    /// Check if the license is in grace period.
    pub fn is_in_grace_period(&self) -> bool {
        if let Some(grace_end) = self.grace_period_ends_at {
            self.status == "suspended" && grace_end > Utc::now().naive_utc()
        } else {
            false
        }
    }

    /// Check if the license is valid for use.
    pub fn is_valid(&self) -> bool {
        self.status == "active" && !self.is_expired() && self.is_blacklisted != Some(true)
    }
}

/// Represents a license binding history record for audit trail.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LicenseBindingHistory {
    pub id: i64,
    pub license_id: String,
    pub action: String,
    pub hardware_id: Option<String>,
    pub device_name: Option<String>,
    pub device_info: Option<String>,
    pub performed_by: Option<String>,
    pub reason: Option<String>,
    pub created_at: NaiveDateTime,
}

/// Actions for license binding history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingAction {
    Bind,
    Release,
    AdminRelease,
    SystemRelease,
}

impl BindingAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            BindingAction::Bind => "bind",
            BindingAction::Release => "release",
            BindingAction::AdminRelease => "admin_release",
            BindingAction::SystemRelease => "system_release",
        }
    }
}

/// Who performed the binding action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformedBy {
    Client,
    Admin,
    System,
}

impl PerformedBy {
    pub fn as_str(&self) -> &'static str {
        match self {
            PerformedBy::Client => "client",
            PerformedBy::Admin => "admin",
            PerformedBy::System => "system",
        }
    }
}

/// Unified database abstraction over SQLite and Postgres.
///
/// Available variants depend on enabled features:
/// - `sqlite` feature enables `Database::SQLite`
/// - `postgres` feature enables `Database::Postgres`
#[derive(Debug, Clone)]
pub enum Database {
    #[cfg(feature = "sqlite")]
    SQLite(SqlitePool),
    #[cfg(feature = "postgres")]
    Postgres(PgPool),
}

impl Database {
    /// Initialize the database connection based on configuration.
    ///
    /// Uses the global configuration from `config.toml` and environment variables.
    /// See `crate::config` for configuration options.
    pub async fn new() -> LicenseResult<Arc<Self>> {
        let config = get_config()?;
        let db_config = &config.database;

        match db_config.db_type.as_str() {
            #[cfg(feature = "sqlite")]
            "sqlite" => {
                let pool = SqlitePool::connect(&db_config.sqlite_url)
                    .await
                    .map_err(|e| {
                        error!("Failed to connect to SQLite: {e}");
                        LicenseError::ServerError(format!("failed to connect to SQLite: {e}"))
                    })?;

                Ok(Arc::new(Database::SQLite(pool)))
            }
            #[cfg(not(feature = "sqlite"))]
            "sqlite" => Err(LicenseError::ConfigError(
                "SQLite support not compiled in. Enable the 'sqlite' feature.".to_string(),
            )),
            #[cfg(feature = "postgres")]
            "postgres" => {
                let pool = PgPool::connect(&db_config.postgres_url)
                    .await
                    .map_err(|e| {
                        error!("Failed to connect to PostgreSQL: {e}");
                        LicenseError::ServerError(format!("failed to connect to PostgreSQL: {e}"))
                    })?;

                Ok(Arc::new(Database::Postgres(pool)))
            }
            #[cfg(not(feature = "postgres"))]
            "postgres" => Err(LicenseError::ConfigError(
                "PostgreSQL support not compiled in. Enable the 'postgres' feature.".to_string(),
            )),
            other => Err(LicenseError::ConfigError(format!(
                "unsupported database type: {other}"
            ))),
        }
    }

    /// Insert a new license or update an existing one.
    ///
    /// This acts like an "upsert" keyed on `license_id`:
    /// - if the license doesn't exist, it is created
    /// - if it exists, the fields are updated
    pub async fn insert_license(&self, license: License) -> LicenseResult<()> {
        match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                query(
                    r#"
                    INSERT INTO licenses (
                        license_id, client_id, status, features, issued_at, expires_at,
                        hardware_id, signature, last_heartbeat, org_id, org_name,
                        license_key, tier, device_name, device_info, bound_at,
                        last_seen_at, suspended_at, revoked_at, revoke_reason,
                        grace_period_ends_at, suspension_message, is_blacklisted,
                        blacklisted_at, blacklist_reason, metadata,
                        bandwidth_used_bytes, bandwidth_limit_bytes, quota_exceeded
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(license_id) DO UPDATE SET
                        client_id            = excluded.client_id,
                        status               = excluded.status,
                        features             = excluded.features,
                        issued_at            = excluded.issued_at,
                        expires_at           = excluded.expires_at,
                        hardware_id          = excluded.hardware_id,
                        signature            = excluded.signature,
                        last_heartbeat       = excluded.last_heartbeat,
                        org_id               = excluded.org_id,
                        org_name             = excluded.org_name,
                        license_key          = excluded.license_key,
                        tier                 = excluded.tier,
                        device_name          = excluded.device_name,
                        device_info          = excluded.device_info,
                        bound_at             = excluded.bound_at,
                        last_seen_at         = excluded.last_seen_at,
                        suspended_at         = excluded.suspended_at,
                        revoked_at           = excluded.revoked_at,
                        revoke_reason        = excluded.revoke_reason,
                        grace_period_ends_at = excluded.grace_period_ends_at,
                        suspension_message   = excluded.suspension_message,
                        is_blacklisted       = excluded.is_blacklisted,
                        blacklisted_at       = excluded.blacklisted_at,
                        blacklist_reason     = excluded.blacklist_reason,
                        metadata             = excluded.metadata,
                        bandwidth_used_bytes = excluded.bandwidth_used_bytes,
                        bandwidth_limit_bytes = excluded.bandwidth_limit_bytes,
                        quota_exceeded       = excluded.quota_exceeded
                    "#,
                )
                .bind(&license.license_id)
                .bind(&license.client_id)
                .bind(&license.status)
                .bind(&license.features)
                .bind(license.issued_at)
                .bind(license.expires_at)
                .bind(&license.hardware_id)
                .bind(&license.signature)
                .bind(license.last_heartbeat)
                .bind(&license.org_id)
                .bind(&license.org_name)
                .bind(&license.license_key)
                .bind(&license.tier)
                .bind(&license.device_name)
                .bind(&license.device_info)
                .bind(license.bound_at)
                .bind(license.last_seen_at)
                .bind(license.suspended_at)
                .bind(license.revoked_at)
                .bind(&license.revoke_reason)
                .bind(license.grace_period_ends_at)
                .bind(&license.suspension_message)
                .bind(license.is_blacklisted)
                .bind(license.blacklisted_at)
                .bind(&license.blacklist_reason)
                .bind(&license.metadata)
                .bind(license.bandwidth_used_bytes)
                .bind(license.bandwidth_limit_bytes)
                .bind(license.quota_exceeded)
                .execute(pool)
                .await
                .map_err(|e| {
                    error!("SQLite insert_license failed: {e}");
                    LicenseError::ServerError(format!("database error: {e}"))
                })?;
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                query(
                    r#"
                    INSERT INTO licenses (
                        license_id, client_id, status, features, issued_at, expires_at,
                        hardware_id, signature, last_heartbeat, org_id, org_name,
                        license_key, tier, device_name, device_info, bound_at,
                        last_seen_at, suspended_at, revoked_at, revoke_reason,
                        grace_period_ends_at, suspension_message, is_blacklisted,
                        blacklisted_at, blacklist_reason, metadata,
                        bandwidth_used_bytes, bandwidth_limit_bytes, quota_exceeded
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29)
                    ON CONFLICT (license_id) DO UPDATE SET
                        client_id            = EXCLUDED.client_id,
                        status               = EXCLUDED.status,
                        features             = EXCLUDED.features,
                        issued_at            = EXCLUDED.issued_at,
                        expires_at           = EXCLUDED.expires_at,
                        hardware_id          = EXCLUDED.hardware_id,
                        signature            = EXCLUDED.signature,
                        last_heartbeat       = EXCLUDED.last_heartbeat,
                        org_id               = EXCLUDED.org_id,
                        org_name             = EXCLUDED.org_name,
                        license_key          = EXCLUDED.license_key,
                        tier                 = EXCLUDED.tier,
                        device_name          = EXCLUDED.device_name,
                        device_info          = EXCLUDED.device_info,
                        bound_at             = EXCLUDED.bound_at,
                        last_seen_at         = EXCLUDED.last_seen_at,
                        suspended_at         = EXCLUDED.suspended_at,
                        revoked_at           = EXCLUDED.revoked_at,
                        revoke_reason        = EXCLUDED.revoke_reason,
                        grace_period_ends_at = EXCLUDED.grace_period_ends_at,
                        suspension_message   = EXCLUDED.suspension_message,
                        is_blacklisted       = EXCLUDED.is_blacklisted,
                        blacklisted_at       = EXCLUDED.blacklisted_at,
                        blacklist_reason     = EXCLUDED.blacklist_reason,
                        metadata             = EXCLUDED.metadata,
                        bandwidth_used_bytes = EXCLUDED.bandwidth_used_bytes,
                        bandwidth_limit_bytes = EXCLUDED.bandwidth_limit_bytes,
                        quota_exceeded       = EXCLUDED.quota_exceeded
                    "#,
                )
                .bind(&license.license_id)
                .bind(&license.client_id)
                .bind(&license.status)
                .bind(&license.features)
                .bind(license.issued_at)
                .bind(license.expires_at)
                .bind(&license.hardware_id)
                .bind(&license.signature)
                .bind(license.last_heartbeat)
                .bind(&license.org_id)
                .bind(&license.org_name)
                .bind(&license.license_key)
                .bind(&license.tier)
                .bind(&license.device_name)
                .bind(&license.device_info)
                .bind(license.bound_at)
                .bind(license.last_seen_at)
                .bind(license.suspended_at)
                .bind(license.revoked_at)
                .bind(&license.revoke_reason)
                .bind(license.grace_period_ends_at)
                .bind(&license.suspension_message)
                .bind(license.is_blacklisted)
                .bind(license.blacklisted_at)
                .bind(&license.blacklist_reason)
                .bind(&license.metadata)
                .bind(license.bandwidth_used_bytes)
                .bind(license.bandwidth_limit_bytes)
                .bind(license.quota_exceeded)
                .execute(pool)
                .await
                .map_err(|e| {
                    error!("Postgres insert_license failed: {e}");
                    LicenseError::ServerError(format!("database error: {e}"))
                })?;
            }
        }

        Ok(())
    }

    /// Fetch a license by its ID.
    ///
    /// Returns:
    /// - `Ok(Some(License))` if found
    /// - `Ok(None)` if not found
    /// - `Err(LicenseError::ServerError)` on DB failure
    pub async fn get_license(&self, license_id: &str) -> LicenseResult<Option<License>> {
        match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                let license = query_as::<_, License>("SELECT * FROM licenses WHERE license_id = ?")
                    .bind(license_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        error!("SQLite get_license failed: {e}");
                        LicenseError::ServerError(format!("database error: {e}"))
                    })?;

                Ok(license)
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                let license =
                    query_as::<_, License>("SELECT * FROM licenses WHERE license_id = $1")
                        .bind(license_id)
                        .fetch_optional(pool)
                        .await
                        .map_err(|e| {
                            error!("Postgres get_license failed: {e}");
                            LicenseError::ServerError(format!("database error: {e}"))
                        })?;

                Ok(license)
            }
        }
    }

    /// Update the `last_heartbeat` timestamp for a license/client pair.
    ///
    /// Returns:
    /// - `Ok(true)` if a row was updated
    /// - `Ok(false)` if no matching row was found
    /// - `Err(LicenseError::ServerError)` on DB failure
    pub async fn update_last_heartbeat(
        &self,
        license_id: &str,
        client_id: &str,
    ) -> LicenseResult<bool> {
        let now = Utc::now().naive_utc();

        let rows_affected = match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => query(
                "UPDATE licenses \
                     SET last_heartbeat = ? \
                     WHERE license_id = ? AND client_id = ?",
            )
            .bind(now)
            .bind(license_id)
            .bind(client_id)
            .execute(pool)
            .await
            .map_err(|e| {
                error!("SQLite update_last_heartbeat failed: {e}");
                LicenseError::ServerError(format!("database error: {e}"))
            })?
            .rows_affected(),
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => query(
                "UPDATE licenses \
                     SET last_heartbeat = $1 \
                     WHERE license_id = $2 AND client_id = $3",
            )
            .bind(now)
            .bind(license_id)
            .bind(client_id)
            .execute(pool)
            .await
            .map_err(|e| {
                error!("Postgres update_last_heartbeat failed: {e}");
                LicenseError::ServerError(format!("database error: {e}"))
            })?
            .rows_affected(),
        };

        Ok(rows_affected > 0)
    }

    /// Fetch a license by its human-readable license key.
    ///
    /// Returns:
    /// - `Ok(Some(License))` if found
    /// - `Ok(None)` if not found
    /// - `Err(LicenseError::ServerError)` on DB failure
    pub async fn get_license_by_key(&self, license_key: &str) -> LicenseResult<Option<License>> {
        match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                let license =
                    query_as::<_, License>("SELECT * FROM licenses WHERE license_key = ?")
                        .bind(license_key)
                        .fetch_optional(pool)
                        .await
                        .map_err(|e| {
                            error!("SQLite get_license_by_key failed: {e}");
                            LicenseError::ServerError(format!("database error: {e}"))
                        })?;

                Ok(license)
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                let license =
                    query_as::<_, License>("SELECT * FROM licenses WHERE license_key = $1")
                        .bind(license_key)
                        .fetch_optional(pool)
                        .await
                        .map_err(|e| {
                            error!("Postgres get_license_by_key failed: {e}");
                            LicenseError::ServerError(format!("database error: {e}"))
                        })?;

                Ok(license)
            }
        }
    }

    /// Check if a license key already exists in the database.
    pub async fn license_key_exists(&self, license_key: &str) -> LicenseResult<bool> {
        match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                let result: (i64,) =
                    query_as("SELECT COUNT(*) FROM licenses WHERE license_key = ?")
                        .bind(license_key)
                        .fetch_one(pool)
                        .await
                        .map_err(|e| {
                            error!("SQLite license_key_exists failed: {e}");
                            LicenseError::ServerError(format!("database error: {e}"))
                        })?;

                Ok(result.0 > 0)
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                let result: (i64,) =
                    query_as("SELECT COUNT(*) FROM licenses WHERE license_key = $1")
                        .bind(license_key)
                        .fetch_one(pool)
                        .await
                        .map_err(|e| {
                            error!("Postgres license_key_exists failed: {e}");
                            LicenseError::ServerError(format!("database error: {e}"))
                        })?;

                Ok(result.0 > 0)
            }
        }
    }

    /// List licenses by organization ID.
    pub async fn list_licenses_by_org(&self, org_id: &str) -> LicenseResult<Vec<License>> {
        match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                let licenses = query_as::<_, License>("SELECT * FROM licenses WHERE org_id = ?")
                    .bind(org_id)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| {
                        error!("SQLite list_licenses_by_org failed: {e}");
                        LicenseError::ServerError(format!("database error: {e}"))
                    })?;

                Ok(licenses)
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                let licenses = query_as::<_, License>("SELECT * FROM licenses WHERE org_id = $1")
                    .bind(org_id)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| {
                        error!("Postgres list_licenses_by_org failed: {e}");
                        LicenseError::ServerError(format!("database error: {e}"))
                    })?;

                Ok(licenses)
            }
        }
    }

    /// Update license status.
    pub async fn update_license_status(
        &self,
        license_id: &str,
        status: &str,
    ) -> LicenseResult<bool> {
        let rows_affected = match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => query("UPDATE licenses SET status = ? WHERE license_id = ?")
                .bind(status)
                .bind(license_id)
                .execute(pool)
                .await
                .map_err(|e| {
                    error!("SQLite update_license_status failed: {e}");
                    LicenseError::ServerError(format!("database error: {e}"))
                })?
                .rows_affected(),
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                query("UPDATE licenses SET status = $1 WHERE license_id = $2")
                    .bind(status)
                    .bind(license_id)
                    .execute(pool)
                    .await
                    .map_err(|e| {
                        error!("Postgres update_license_status failed: {e}");
                        LicenseError::ServerError(format!("database error: {e}"))
                    })?
                    .rows_affected()
            }
        };

        Ok(rows_affected > 0)
    }

    /// Bind a license to hardware.
    pub async fn bind_license(
        &self,
        license_id: &str,
        hardware_id: &str,
        device_name: Option<&str>,
        device_info: Option<&str>,
    ) -> LicenseResult<bool> {
        let now = Utc::now().naive_utc();

        let rows_affected = match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => query(
                "UPDATE licenses SET \
                     hardware_id = ?, \
                     device_name = ?, \
                     device_info = ?, \
                     bound_at = ?, \
                     last_seen_at = ? \
                 WHERE license_id = ?",
            )
            .bind(hardware_id)
            .bind(device_name)
            .bind(device_info)
            .bind(now)
            .bind(now)
            .bind(license_id)
            .execute(pool)
            .await
            .map_err(|e| {
                error!("SQLite bind_license failed: {e}");
                LicenseError::ServerError(format!("database error: {e}"))
            })?
            .rows_affected(),
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => query(
                "UPDATE licenses SET \
                     hardware_id = $1, \
                     device_name = $2, \
                     device_info = $3, \
                     bound_at = $4, \
                     last_seen_at = $5 \
                 WHERE license_id = $6",
            )
            .bind(hardware_id)
            .bind(device_name)
            .bind(device_info)
            .bind(now)
            .bind(now)
            .bind(license_id)
            .execute(pool)
            .await
            .map_err(|e| {
                error!("Postgres bind_license failed: {e}");
                LicenseError::ServerError(format!("database error: {e}"))
            })?
            .rows_affected(),
        };

        Ok(rows_affected > 0)
    }

    /// Release a license from hardware binding.
    pub async fn release_license(&self, license_id: &str) -> LicenseResult<bool> {
        let rows_affected = match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => query(
                "UPDATE licenses SET \
                     hardware_id = NULL, \
                     device_name = NULL, \
                     device_info = NULL, \
                     bound_at = NULL \
                 WHERE license_id = ?",
            )
            .bind(license_id)
            .execute(pool)
            .await
            .map_err(|e| {
                error!("SQLite release_license failed: {e}");
                LicenseError::ServerError(format!("database error: {e}"))
            })?
            .rows_affected(),
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => query(
                "UPDATE licenses SET \
                     hardware_id = NULL, \
                     device_name = NULL, \
                     device_info = NULL, \
                     bound_at = NULL \
                 WHERE license_id = $1",
            )
            .bind(license_id)
            .execute(pool)
            .await
            .map_err(|e| {
                error!("Postgres release_license failed: {e}");
                LicenseError::ServerError(format!("database error: {e}"))
            })?
            .rows_affected(),
        };

        Ok(rows_affected > 0)
    }

    /// Record a binding action in the history table.
    #[allow(clippy::too_many_arguments)]
    pub async fn record_binding_history(
        &self,
        license_id: &str,
        action: BindingAction,
        hardware_id: Option<&str>,
        device_name: Option<&str>,
        device_info: Option<&str>,
        performed_by: PerformedBy,
        reason: Option<&str>,
    ) -> LicenseResult<()> {
        match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                query(
                    "INSERT INTO license_binding_history \
                         (license_id, action, hardware_id, device_name, device_info, performed_by, reason) \
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(license_id)
                .bind(action.as_str())
                .bind(hardware_id)
                .bind(device_name)
                .bind(device_info)
                .bind(performed_by.as_str())
                .bind(reason)
                .execute(pool)
                .await
                .map_err(|e| {
                    error!("SQLite record_binding_history failed: {e}");
                    LicenseError::ServerError(format!("database error: {e}"))
                })?;
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                query(
                    "INSERT INTO license_binding_history \
                         (license_id, action, hardware_id, device_name, device_info, performed_by, reason) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7)",
                )
                .bind(license_id)
                .bind(action.as_str())
                .bind(hardware_id)
                .bind(device_name)
                .bind(device_info)
                .bind(performed_by.as_str())
                .bind(reason)
                .execute(pool)
                .await
                .map_err(|e| {
                    error!("Postgres record_binding_history failed: {e}");
                    LicenseError::ServerError(format!("database error: {e}"))
                })?;
            }
        }

        Ok(())
    }

    /// Update last_seen_at timestamp for a license.
    pub async fn update_last_seen(&self, license_id: &str) -> LicenseResult<bool> {
        let now = Utc::now().naive_utc();

        let rows_affected = match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                query("UPDATE licenses SET last_seen_at = ? WHERE license_id = ?")
                    .bind(now)
                    .bind(license_id)
                    .execute(pool)
                    .await
                    .map_err(|e| {
                        error!("SQLite update_last_seen failed: {e}");
                        LicenseError::ServerError(format!("database error: {e}"))
                    })?
                    .rows_affected()
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                query("UPDATE licenses SET last_seen_at = $1 WHERE license_id = $2")
                    .bind(now)
                    .bind(license_id)
                    .execute(pool)
                    .await
                    .map_err(|e| {
                        error!("Postgres update_last_seen failed: {e}");
                        LicenseError::ServerError(format!("database error: {e}"))
                    })?
                    .rows_affected()
            }
        };

        Ok(rows_affected > 0)
    }

    /// Get licenses with expired grace periods (suspended licenses past their grace_period_ends_at).
    pub async fn get_expired_grace_period_licenses(
        &self,
        now: NaiveDateTime,
    ) -> LicenseResult<Vec<License>> {
        match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                let licenses: Vec<License> = query_as(
                    "SELECT * FROM licenses WHERE status = 'suspended' AND grace_period_ends_at IS NOT NULL AND grace_period_ends_at < ?"
                )
                .bind(now)
                .fetch_all(pool)
                .await
                .map_err(|e| {
                    error!("SQLite get_expired_grace_period_licenses failed: {e}");
                    LicenseError::ServerError(format!("database error: {e}"))
                })?;

                Ok(licenses)
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                let licenses: Vec<License> = query_as(
                    "SELECT * FROM licenses WHERE status = 'suspended' AND grace_period_ends_at IS NOT NULL AND grace_period_ends_at < $1"
                )
                .bind(now)
                .fetch_all(pool)
                .await
                .map_err(|e| {
                    error!("Postgres get_expired_grace_period_licenses failed: {e}");
                    LicenseError::ServerError(format!("database error: {e}"))
                })?;

                Ok(licenses)
            }
        }
    }

    /// Get expired licenses (active licenses past their expires_at).
    pub async fn get_expired_licenses(&self, now: NaiveDateTime) -> LicenseResult<Vec<License>> {
        match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                let licenses: Vec<License> = query_as(
                    "SELECT * FROM licenses WHERE status = 'active' AND expires_at IS NOT NULL AND expires_at < ?",
                )
                .bind(now)
                .fetch_all(pool)
                .await
                .map_err(|e| {
                    error!("SQLite get_expired_licenses failed: {e}");
                    LicenseError::ServerError(format!("database error: {e}"))
                })?;

                Ok(licenses)
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                let licenses: Vec<License> = query_as(
                    "SELECT * FROM licenses WHERE status = 'active' AND expires_at IS NOT NULL AND expires_at < $1",
                )
                .bind(now)
                .fetch_all(pool)
                .await
                .map_err(|e| {
                    error!("Postgres get_expired_licenses failed: {e}");
                    LicenseError::ServerError(format!("database error: {e}"))
                })?;

                Ok(licenses)
            }
        }
    }

    /// Get licenses bound to stale devices (not seen since threshold).
    pub async fn get_stale_device_licenses(
        &self,
        threshold: NaiveDateTime,
    ) -> LicenseResult<Vec<License>> {
        match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                let licenses: Vec<License> = query_as(
                    "SELECT * FROM licenses WHERE hardware_id IS NOT NULL AND last_seen_at IS NOT NULL AND last_seen_at < ?",
                )
                .bind(threshold)
                .fetch_all(pool)
                .await
                .map_err(|e| {
                    error!("SQLite get_stale_device_licenses failed: {e}");
                    LicenseError::ServerError(format!("database error: {e}"))
                })?;

                Ok(licenses)
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                let licenses: Vec<License> = query_as(
                    "SELECT * FROM licenses WHERE hardware_id IS NOT NULL AND last_seen_at IS NOT NULL AND last_seen_at < $1",
                )
                .bind(threshold)
                .fetch_all(pool)
                .await
                .map_err(|e| {
                    error!("Postgres get_stale_device_licenses failed: {e}");
                    LicenseError::ServerError(format!("database error: {e}"))
                })?;

                Ok(licenses)
            }
        }
    }

    /// Update usage/quota fields for a license.
    ///
    /// Updates bandwidth_used_bytes, bandwidth_limit_bytes, and quota_exceeded.
    pub async fn update_usage(
        &self,
        license_id: &str,
        bandwidth_used_bytes: i64,
        bandwidth_limit_bytes: Option<i64>,
        quota_exceeded: bool,
    ) -> LicenseResult<bool> {
        let rows_affected = match self {
            #[cfg(feature = "sqlite")]
            Database::SQLite(pool) => {
                query(
                    "UPDATE licenses SET \
                         bandwidth_used_bytes = ?, \
                         bandwidth_limit_bytes = ?, \
                         quota_exceeded = ? \
                     WHERE license_id = ?",
                )
                .bind(bandwidth_used_bytes)
                .bind(bandwidth_limit_bytes)
                .bind(quota_exceeded)
                .bind(license_id)
                .execute(pool)
                .await
                .map_err(|e| {
                    error!("SQLite update_usage failed: {e}");
                    LicenseError::ServerError(format!("database error: {e}"))
                })?
                .rows_affected()
            }
            #[cfg(feature = "postgres")]
            Database::Postgres(pool) => {
                query(
                    "UPDATE licenses SET \
                         bandwidth_used_bytes = $1, \
                         bandwidth_limit_bytes = $2, \
                         quota_exceeded = $3 \
                     WHERE license_id = $4",
                )
                .bind(bandwidth_used_bytes)
                .bind(bandwidth_limit_bytes)
                .bind(quota_exceeded)
                .bind(license_id)
                .execute(pool)
                .await
                .map_err(|e| {
                    error!("Postgres update_usage failed: {e}");
                    LicenseError::ServerError(format!("database error: {e}"))
                })?
                .rows_affected()
            }
        };

        Ok(rows_affected > 0)
    }
}
