use sqlx::{query, query_as, Pool, SqlitePool, PgPool, FromRow};
use config::Config;
use chrono::{NaiveDateTime, Utc};
use std::sync::Arc;
use tracing::error;

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
}

#[derive(Debug, Clone)]
pub enum Database {
    SQLite(SqlitePool),
    Postgres(PgPool),
}

impl Database {
    /// Initialize the database connection based on the configuration
    pub async fn new() -> Arc<Self> {
        let settings = Config::builder()
            .add_source(config::File::with_name("config"))
            .build()
            .expect("Failed to load configuration");

        let db_type: String = settings.get("database.db_type").unwrap();
        match db_type.as_str() {
            "sqlite" => {
                let sqlite_url: String = settings.get("database.sqlite_url").unwrap();
                let pool = SqlitePool::connect(&sqlite_url)
                    .await
                    .expect("Failed to connect to SQLite");
                Arc::new(Database::SQLite(pool))
            }
            "postgres" => {
                let postgres_url: String = settings.get("database.postgres_url").unwrap();
                let pool = PgPool::connect(&postgres_url)
                    .await
                    .expect("Failed to connect to PostgreSQL");
                Arc::new(Database::Postgres(pool))
            }
            _ => panic!("Unsupported database type"),
        }
    }

/// Insert a new license or update an existing one in the database
pub async fn insert_license(&self, license: License) -> Result<(), sqlx::Error> {
    match self {
        Database::SQLite(pool) => {
            query(
                r#"
                INSERT INTO licenses (license_id, client_id, status, features, issued_at, expires_at, hardware_id, signature)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(license_id) DO UPDATE SET
                    client_id = excluded.client_id,
                    status = excluded.status,
                    features = excluded.features,
                    issued_at = excluded.issued_at,
                    expires_at = excluded.expires_at,
                    hardware_id = excluded.hardware_id,
                    signature = excluded.signature
                "#
            )
            .bind(&license.license_id)
            .bind(&license.client_id)
            .bind(&license.status)
            .bind(&license.features)
            .bind(license.issued_at)
            .bind(license.expires_at)
            .bind(&license.hardware_id)
            .bind(&license.signature)
            .execute(pool)
            .await?;
        }
        Database::Postgres(pool) => {
            query(
                r#"
                INSERT INTO licenses (license_id, client_id, status, features, issued_at, expires_at, hardware_id, signature)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (license_id) DO UPDATE SET
                    client_id = EXCLUDED.client_id,
                    status = EXCLUDED.status,
                    features = EXCLUDED.features,
                    issued_at = EXCLUDED.issued_at,
                    expires_at = EXCLUDED.expires_at,
                    hardware_id = EXCLUDED.hardware_id,
                    signature = EXCLUDED.signature
                "#
            )
            .bind(&license.license_id)
            .bind(&license.client_id)
            .bind(&license.status)
            .bind(&license.features)
            .bind(license.issued_at)
            .bind(license.expires_at)
            .bind(&license.hardware_id)
            .bind(&license.signature)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

    /// Fetch a license by its ID
    pub async fn get_license(&self, license_id: &str) -> Result<Option<License>, sqlx::Error> {
        match self {
            Database::SQLite(pool) => {
                let license = query_as::<_, License>(
                    "SELECT * FROM licenses WHERE license_id = ?"
                )
                .bind(license_id)
                .fetch_optional(pool)
                .await?;
                Ok(license)
            }
            Database::Postgres(pool) => {
                let license = query_as::<_, License>(
                    "SELECT * FROM licenses WHERE license_id = $1"
                )
                .bind(license_id)
                .fetch_optional(pool)
                .await?;
                Ok(license)
            }
        }
    }
}
