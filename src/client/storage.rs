//! Secure storage abstraction for license and cache data.
//!
//! This module provides a unified storage interface that:
//! 1. Tries OS keyring first (most secure)
//! 2. Falls back to file storage in app data directory
//! 3. Handles migration from legacy CWD-based files
//!
//! ## Storage Locations
//!
//! **Keyring (Primary):**
//! - Service: `talos`
//! - Keys: `license:{hardware_id}` and `cache:{hardware_id}`
//!
//! **File Fallback (Secondary):**
//! - Windows: `%APPDATA%\talos\`
//! - macOS: `~/Library/Application Support/talos/`
//! - Linux: `~/.local/share/talos/`
//!
//! ## Migration
//!
//! On load, if data is not found in keyring or app data directory,
//! the module checks for legacy files in the current working directory
//! and automatically migrates them.

use crate::errors::{LicenseError, LicenseResult};
use crate::hardware::get_hardware_id;

use std::io::ErrorKind;
use std::path::PathBuf;
use tokio::fs;

/// File names for stored data.
const LICENSE_FILE: &str = "talos_license.enc";
const CACHE_FILE: &str = "talos_cache.enc";

/// Service name for keyring storage.
const KEYRING_SERVICE: &str = "talos";

/// Identifies what type of data is being stored.
#[derive(Debug, Clone, Copy)]
pub enum StorageKey {
    License,
    Cache,
}

impl StorageKey {
    /// Get the keyring entry name for this storage key.
    fn keyring_name(&self) -> String {
        let hw_id = get_hardware_id();
        match self {
            StorageKey::License => format!("license:{}", hw_id),
            StorageKey::Cache => format!("cache:{}", hw_id),
        }
    }

    /// Get the filename for file-based storage.
    fn filename(&self) -> &'static str {
        match self {
            StorageKey::License => LICENSE_FILE,
            StorageKey::Cache => CACHE_FILE,
        }
    }
}

/// Get the application data directory for talos.
///
/// Returns platform-specific paths:
/// - Windows: `%APPDATA%\talos\`
/// - macOS: `~/Library/Application Support/talos/`
/// - Linux: `~/.local/share/talos/`
fn get_app_data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|p| p.join("talos"))
}

/// Get the full path for file-based storage.
fn get_storage_path(key: StorageKey) -> Option<PathBuf> {
    get_app_data_dir().map(|dir| dir.join(key.filename()))
}

/// Get the legacy (CWD-based) path for migration.
fn get_legacy_path(key: StorageKey) -> PathBuf {
    PathBuf::from(key.filename())
}

// === Keyring Operations ===

/// Save data to the OS keyring.
fn save_to_keyring(key: StorageKey, data: &str) -> Result<(), keyring::Error> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &key.keyring_name())?;
    entry.set_password(data)
}

/// Load data from the OS keyring.
fn load_from_keyring(key: StorageKey) -> Result<String, keyring::Error> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &key.keyring_name())?;
    entry.get_password()
}

/// Delete data from the OS keyring.
fn clear_from_keyring(key: StorageKey) -> Result<(), keyring::Error> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &key.keyring_name())?;
    entry.delete_credential()
}

// === File Operations ===

/// Save data to file in app data directory.
async fn save_to_file(key: StorageKey, data: &str) -> LicenseResult<()> {
    let dir = get_app_data_dir().ok_or_else(|| {
        LicenseError::StorageError(std::io::Error::new(
            ErrorKind::NotFound,
            "Could not determine app data directory",
        ))
    })?;

    // Ensure directory exists
    fs::create_dir_all(&dir).await?;

    let path = dir.join(key.filename());
    fs::write(&path, data).await?;
    Ok(())
}

/// Load data from file in app data directory.
async fn load_from_file(key: StorageKey) -> LicenseResult<String> {
    let path = get_storage_path(key).ok_or_else(|| {
        LicenseError::StorageError(std::io::Error::new(
            ErrorKind::NotFound,
            "Could not determine app data directory",
        ))
    })?;

    match fs::read_to_string(&path).await {
        Ok(data) => Ok(data),
        Err(e) if e.kind() == ErrorKind::NotFound => Err(LicenseError::InvalidLicense(
            "No stored data found.".to_string(),
        )),
        Err(e) => Err(LicenseError::StorageError(e)),
    }
}

/// Delete file from app data directory.
async fn clear_from_file(key: StorageKey) -> LicenseResult<()> {
    if let Some(path) = get_storage_path(key) {
        match fs::remove_file(&path).await {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(LicenseError::StorageError(e)),
        }
    } else {
        Ok(())
    }
}

// === Legacy Migration ===

/// Check for and load data from legacy CWD-based file.
async fn load_from_legacy(key: StorageKey) -> Option<String> {
    let path = get_legacy_path(key);
    fs::read_to_string(&path).await.ok()
}

/// Delete legacy CWD-based file after migration.
async fn clear_legacy_file(key: StorageKey) {
    let path = get_legacy_path(key);
    let _ = fs::remove_file(&path).await;
}

// === Public API ===

/// Save encrypted data to storage.
///
/// Tries keyring first, falls back to file storage if keyring fails.
pub async fn save_to_storage(key: StorageKey, data: &str) -> LicenseResult<()> {
    // Try keyring first
    match save_to_keyring(key, data) {
        Ok(()) => {
            log::debug!("Saved {:?} to keyring", key);
            // Verify the save worked by reading back
            if load_from_keyring(key).is_ok() {
                return Ok(());
            }
            log::debug!("Keyring save verification failed for {:?}, falling back to file", key);
        }
        Err(e) => {
            log::debug!("Keyring save failed for {:?}: {}, falling back to file", key, e);
        }
    }

    // Fall back to file storage
    save_to_file(key, data).await?;
    log::debug!("Saved {:?} to app data directory", key);
    Ok(())
}

/// Load encrypted data from storage.
///
/// Checks in order:
/// 1. Keyring
/// 2. App data directory file
/// 3. Legacy CWD file (with automatic migration)
pub async fn load_from_storage(key: StorageKey) -> LicenseResult<String> {
    // 1. Try keyring
    match load_from_keyring(key) {
        Ok(data) => {
            log::debug!("Loaded {:?} from keyring", key);
            return Ok(data);
        }
        Err(e) => {
            log::debug!("Keyring load failed for {:?}: {}", key, e);
        }
    }

    // 2. Try app data directory
    match load_from_file(key).await {
        Ok(data) => {
            log::debug!("Loaded {:?} from app data directory", key);
            // Try to migrate to keyring for next time
            if save_to_keyring(key, &data).is_ok() {
                log::debug!("Migrated {:?} from app data to keyring", key);
            }
            return Ok(data);
        }
        Err(LicenseError::InvalidLicense(_)) => {
            // Not found, continue to legacy check
        }
        Err(e) => {
            log::debug!("App data file load failed for {:?}: {}", key, e);
        }
    }

    // 3. Try legacy CWD file (migration path)
    if let Some(data) = load_from_legacy(key).await {
        log::info!("Found legacy {:?} file in CWD, migrating...", key);

        // Migrate to new storage
        if let Err(e) = save_to_storage(key, &data).await {
            log::warn!("Failed to migrate {:?} to new storage: {}", key, e);
            // Still return the data even if migration failed
        } else {
            // Migration successful, delete legacy file
            clear_legacy_file(key).await;
            log::info!("Successfully migrated {:?} and cleaned up legacy file", key);
        }

        return Ok(data);
    }

    // Not found anywhere
    Err(LicenseError::InvalidLicense(
        "No stored data found.".to_string(),
    ))
}

/// Clear data from all storage locations.
///
/// Clears from keyring, app data directory, and legacy CWD location.
pub async fn clear_from_storage(key: StorageKey) -> LicenseResult<()> {
    let mut last_error: Option<LicenseError> = None;

    // Clear from keyring (ignore "not found" errors)
    if let Err(e) = clear_from_keyring(key) {
        match e {
            keyring::Error::NoEntry => {}
            _ => log::debug!("Failed to clear {:?} from keyring: {}", key, e),
        }
    }

    // Clear from app data directory
    if let Err(e) = clear_from_file(key).await {
        last_error = Some(e);
    }

    // Clear legacy file too (cleanup)
    clear_legacy_file(key).await;

    match last_error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tokio::test as tokio_test;

    // Test file operations in app data directory
    #[tokio_test]
    #[serial]
    async fn test_file_storage_roundtrip() {
        let test_data = "test_encrypted_data_12345";

        // Save to file
        save_to_file(StorageKey::License, test_data)
            .await
            .expect("save should succeed");

        // Load from file
        let loaded = load_from_file(StorageKey::License)
            .await
            .expect("load should succeed");

        assert_eq!(loaded, test_data);

        // Cleanup
        clear_from_file(StorageKey::License)
            .await
            .expect("clear should succeed");
    }

    #[tokio_test]
    #[serial]
    async fn test_missing_file_returns_invalid_license() {
        // Ensure file doesn't exist
        let _ = clear_from_file(StorageKey::Cache).await;

        let result = load_from_file(StorageKey::Cache).await;
        assert!(matches!(result, Err(LicenseError::InvalidLicense(_))));
    }

    #[tokio_test]
    #[serial]
    async fn test_storage_api_roundtrip() {
        let test_data = "storage_api_test_data";

        // Clear any existing data first
        let _ = clear_from_storage(StorageKey::License).await;

        // Save through public API
        save_to_storage(StorageKey::License, test_data)
            .await
            .expect("save should succeed");

        // Load through public API
        let loaded = load_from_storage(StorageKey::License)
            .await
            .expect("load should succeed");

        assert_eq!(loaded, test_data);

        // Cleanup
        clear_from_storage(StorageKey::License)
            .await
            .expect("clear should succeed");
    }

    #[tokio_test]
    #[serial]
    async fn test_legacy_migration() {
        let test_data = "legacy_migration_test_data";
        let legacy_path = get_legacy_path(StorageKey::Cache);

        // Clear any existing data
        let _ = clear_from_storage(StorageKey::Cache).await;

        // Create a legacy file in CWD
        fs::write(&legacy_path, test_data)
            .await
            .expect("creating legacy file should succeed");

        // Load should find and migrate the legacy file
        let loaded = load_from_storage(StorageKey::Cache)
            .await
            .expect("load should succeed");

        assert_eq!(loaded, test_data);

        // Legacy file should be deleted after migration
        assert!(
            !legacy_path.exists(),
            "legacy file should be deleted after migration"
        );

        // Data should still be accessible (now from new storage)
        let loaded_again = load_from_storage(StorageKey::Cache)
            .await
            .expect("load should still succeed after migration");

        assert_eq!(loaded_again, test_data);

        // Cleanup
        let _ = clear_from_storage(StorageKey::Cache).await;
    }

    #[test]
    fn test_storage_key_names() {
        // Just verify the naming patterns are consistent
        let license_name = StorageKey::License.keyring_name();
        let cache_name = StorageKey::Cache.keyring_name();

        assert!(license_name.starts_with("license:"));
        assert!(cache_name.starts_with("cache:"));
        assert_eq!(StorageKey::License.filename(), "talos_license.enc");
        assert_eq!(StorageKey::Cache.filename(), "talos_cache.enc");
    }

    #[test]
    fn test_app_data_dir_exists() {
        // This should work on all platforms
        let dir = get_app_data_dir();
        assert!(dir.is_some(), "app data directory should be determinable");
    }
}
