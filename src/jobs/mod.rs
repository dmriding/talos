//! Background job scheduler for Talos.
//!
//! This module provides scheduled background jobs for license management tasks.
//! Requires the `background-jobs` feature to be enabled.
//!
//! # Available Jobs
//!
//! - **Grace Period Expiration**: Checks for suspended licenses past their grace period
//!   and updates their status to 'revoked'
//!
//! - **License Expiration**: Checks for active licenses past their expiration date
//!   and updates their status to 'expired'
//!
//! - **Stale Device Cleanup** (optional): Releases licenses from devices that haven't
//!   been seen for a configurable period
//!
//! # Usage
//!
//! ```rust,ignore
//! use talos::jobs::{JobScheduler, JobConfig};
//! use talos::server::Database;
//!
//! let db = Database::new().await?;
//! let config = JobConfig::default();
//! let scheduler = JobScheduler::new(db, config).await?;
//! scheduler.start().await?;
//! ```

use chrono::Utc;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler as TokioJobScheduler};
use tracing::{error, info};

use crate::server::database::Database;

mod grace_period;
mod license_expiration;
mod stale_devices;

pub use grace_period::run_grace_period_check;
pub use license_expiration::run_license_expiration_check;
pub use stale_devices::run_stale_device_cleanup;

/// Configuration for background jobs.
#[derive(Debug, Clone)]
pub struct JobConfig {
    /// Cron expression for grace period expiration check (default: every hour at minute 0)
    pub grace_period_cron: String,
    /// Cron expression for license expiration check (default: every hour at minute 15)
    pub license_expiration_cron: String,
    /// Whether stale device cleanup is enabled (default: false)
    pub stale_device_cleanup_enabled: bool,
    /// Cron expression for stale device cleanup (default: daily at 3 AM)
    pub stale_device_cron: String,
    /// Number of days after which a device is considered stale (default: 90)
    pub stale_device_days: u32,
}

impl Default for JobConfig {
    fn default() -> Self {
        Self {
            // Every hour at minute 0
            grace_period_cron: "0 0 * * * *".to_string(),
            // Every hour at minute 15
            license_expiration_cron: "0 15 * * * *".to_string(),
            // Disabled by default
            stale_device_cleanup_enabled: false,
            // Daily at 3 AM
            stale_device_cron: "0 0 3 * * *".to_string(),
            // 90 days
            stale_device_days: 90,
        }
    }
}

/// Background job scheduler for Talos.
pub struct JobScheduler {
    scheduler: TokioJobScheduler,
    db: Arc<Database>,
    config: JobConfig,
}

impl JobScheduler {
    /// Create a new job scheduler.
    pub async fn new(db: Database, config: JobConfig) -> Result<Self, JobError> {
        let scheduler = TokioJobScheduler::new()
            .await
            .map_err(|e| JobError::SchedulerError(e.to_string()))?;

        Ok(Self {
            scheduler,
            db: Arc::new(db),
            config,
        })
    }

    /// Start the job scheduler with all configured jobs.
    pub async fn start(&self) -> Result<(), JobError> {
        info!("Starting Talos job scheduler");

        // Add grace period expiration job
        self.add_grace_period_job().await?;

        // Add license expiration job
        self.add_license_expiration_job().await?;

        // Add stale device cleanup job if enabled
        if self.config.stale_device_cleanup_enabled {
            self.add_stale_device_job().await?;
        }

        // Start the scheduler
        self.scheduler
            .start()
            .await
            .map_err(|e| JobError::SchedulerError(e.to_string()))?;

        info!("Talos job scheduler started successfully");

        Ok(())
    }

    /// Stop the job scheduler.
    pub async fn shutdown(&mut self) -> Result<(), JobError> {
        info!("Shutting down Talos job scheduler");
        self.scheduler
            .shutdown()
            .await
            .map_err(|e| JobError::SchedulerError(e.to_string()))?;
        Ok(())
    }

    /// Add the grace period expiration job.
    async fn add_grace_period_job(&self) -> Result<(), JobError> {
        let db = Arc::clone(&self.db);

        let job = Job::new_async(self.config.grace_period_cron.as_str(), move |_uuid, _l| {
            let db = Arc::clone(&db);
            Box::pin(async move {
                let now = Utc::now().naive_utc();
                info!("Running grace period expiration check at {}", now);

                match run_grace_period_check(&db).await {
                    Ok(count) => {
                        if count > 0 {
                            info!("Grace period check: {} licenses revoked", count);
                        }
                    }
                    Err(e) => {
                        error!("Grace period check failed: {}", e);
                    }
                }
            })
        })
        .map_err(|e| JobError::SchedulerError(e.to_string()))?;

        self.scheduler
            .add(job)
            .await
            .map_err(|e| JobError::SchedulerError(e.to_string()))?;

        info!(
            "Added grace period expiration job (schedule: {})",
            self.config.grace_period_cron
        );

        Ok(())
    }

    /// Add the license expiration job.
    async fn add_license_expiration_job(&self) -> Result<(), JobError> {
        let db = Arc::clone(&self.db);

        let job = Job::new_async(
            self.config.license_expiration_cron.as_str(),
            move |_uuid, _l| {
                let db = Arc::clone(&db);
                Box::pin(async move {
                    let now = Utc::now().naive_utc();
                    info!("Running license expiration check at {}", now);

                    match run_license_expiration_check(&db).await {
                        Ok(count) => {
                            if count > 0 {
                                info!("License expiration check: {} licenses expired", count);
                            }
                        }
                        Err(e) => {
                            error!("License expiration check failed: {}", e);
                        }
                    }
                })
            },
        )
        .map_err(|e| JobError::SchedulerError(e.to_string()))?;

        self.scheduler
            .add(job)
            .await
            .map_err(|e| JobError::SchedulerError(e.to_string()))?;

        info!(
            "Added license expiration job (schedule: {})",
            self.config.license_expiration_cron
        );

        Ok(())
    }

    /// Add the stale device cleanup job.
    async fn add_stale_device_job(&self) -> Result<(), JobError> {
        let db = Arc::clone(&self.db);
        let stale_days = self.config.stale_device_days;

        let job = Job::new_async(self.config.stale_device_cron.as_str(), move |_uuid, _l| {
            let db = Arc::clone(&db);
            Box::pin(async move {
                let now = Utc::now().naive_utc();
                info!("Running stale device cleanup at {}", now);

                match run_stale_device_cleanup(&db, stale_days).await {
                    Ok(count) => {
                        if count > 0 {
                            info!("Stale device cleanup: {} licenses released", count);
                        }
                    }
                    Err(e) => {
                        error!("Stale device cleanup failed: {}", e);
                    }
                }
            })
        })
        .map_err(|e| JobError::SchedulerError(e.to_string()))?;

        self.scheduler
            .add(job)
            .await
            .map_err(|e| JobError::SchedulerError(e.to_string()))?;

        info!(
            "Added stale device cleanup job (schedule: {}, threshold: {} days)",
            self.config.stale_device_cron, self.config.stale_device_days
        );

        Ok(())
    }

    /// Run the grace period check immediately (useful for testing or manual triggers).
    pub async fn run_grace_period_check_now(&self) -> Result<u32, JobError> {
        run_grace_period_check(&self.db).await
    }

    /// Run the license expiration check immediately (useful for testing or manual triggers).
    pub async fn run_license_expiration_check_now(&self) -> Result<u32, JobError> {
        run_license_expiration_check(&self.db).await
    }

    /// Run the stale device cleanup immediately (useful for testing or manual triggers).
    pub async fn run_stale_device_cleanup_now(&self) -> Result<u32, JobError> {
        run_stale_device_cleanup(&self.db, self.config.stale_device_days).await
    }
}

/// Errors that can occur in the job scheduler.
#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("Scheduler error: {0}")]
    SchedulerError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Job execution error: {0}")]
    ExecutionError(String),
}

impl From<crate::errors::LicenseError> for JobError {
    fn from(err: crate::errors::LicenseError) -> Self {
        JobError::DatabaseError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let config = JobConfig::default();
        assert_eq!(config.grace_period_cron, "0 0 * * * *");
        assert_eq!(config.license_expiration_cron, "0 15 * * * *");
        assert!(!config.stale_device_cleanup_enabled);
        assert_eq!(config.stale_device_days, 90);
    }
}
