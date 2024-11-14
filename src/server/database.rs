use sqlx::{Pool, SqlitePool, PgPool, query, query_as, FromRow};
use config::Config;
use chrono::{NaiveDateTime, Utc};
use std::sync::Arc;

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

    // Insert a new license
    pub async fn insert_license(&self, license: License) -> Result<(), sqlx::Error> {
        match self {
            Database::SQLite(pool) => {
                query!(
                    r#"
                    INSERT INTO licenses (license_id, client_id, status, features, issued_at, expires_at, hardware_id, signature)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                    license.license_id,
                    license.client_id,
                    license.status,
                    license.features,
                    license.issued_at,
                    license.expires_at,
                    license.hardware_id,
                    license.signature
                )
                .execute(pool)
                .await?;
            }
            Database::Postgres(pool) => {
                query!(
                    r#"
                    INSERT INTO licenses (license_id, client_id, status, features, issued_at, expires_at, hardware_id, signature)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    "#,
                    license.license_id,
                    license.client_id,
                    license.status,
                    license.features,
                    license.issued_at,
                    license.expires_at,
                    license.hardware_id,
                    license.signature
                )
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }
}
