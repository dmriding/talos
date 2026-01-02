use chrono::{NaiveDateTime, Utc};
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
#[derive(Debug, Clone, FromRow)]
pub struct License {
    pub license_id: String,
    pub client_id: String,
    pub status: String,
    pub features: Option<String>,
    pub issued_at: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
    pub hardware_id: Option<String>,
    pub signature: Option<String>,
    pub last_heartbeat: Option<NaiveDateTime>,
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
                        license_id,
                        client_id,
                        status,
                        features,
                        issued_at,
                        expires_at,
                        hardware_id,
                        signature,
                        last_heartbeat
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(license_id) DO UPDATE SET
                        client_id      = excluded.client_id,
                        status         = excluded.status,
                        features       = excluded.features,
                        issued_at      = excluded.issued_at,
                        expires_at     = excluded.expires_at,
                        hardware_id    = excluded.hardware_id,
                        signature      = excluded.signature,
                        last_heartbeat = excluded.last_heartbeat
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
                        license_id,
                        client_id,
                        status,
                        features,
                        issued_at,
                        expires_at,
                        hardware_id,
                        signature,
                        last_heartbeat
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    ON CONFLICT (license_id) DO UPDATE SET
                        client_id      = EXCLUDED.client_id,
                        status         = EXCLUDED.status,
                        features       = EXCLUDED.features,
                        issued_at      = EXCLUDED.issued_at,
                        expires_at     = EXCLUDED.expires_at,
                        hardware_id    = EXCLUDED.hardware_id,
                        signature      = EXCLUDED.signature,
                        last_heartbeat = EXCLUDED.last_heartbeat
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
}
