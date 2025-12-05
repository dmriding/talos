//! Hardware fingerprinting for Talos license binding.
//!
//! This module unifies platform-specific hardware identification
//! (CPU ID + motherboard/board ID) into a *hashed* fingerprint.
//!
//! Why hashed?
//! - avoids storing raw hardware serials (privacy + security)
//! - normalizes inconsistent values across OSes
//! - allows stable lifetime binding without revealing device secrets

use crate::errors::{LicenseError, LicenseResult};
use ring::digest;

/// Platform-specific hardware ID providers
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;

/// Returns a *consistent hashed fingerprint* based on the device's hardware.
///
/// Steps:
/// 1. Try to retrieve CPU ID (best-effort)
/// 2. Try to retrieve motherboard / board serial (best-effort)
/// 3. Replace missing values with deterministic fallback labels
/// 4. Compute SHA-256 hash over "cpu_id|board_id"
///
/// Example output:
///     "0a3e7c8d9921ac4d89f11223f4d447adfeec2d722f974146ecf2917c4e97fcb2"
pub fn get_hardware_id() -> String {
    // 1. Best-effort raw identifiers
    let cpu_raw = get_cpu_id().unwrap_or_else(|_| "cpu_unknown".to_string());
    let board_raw = get_motherboard_id().unwrap_or_else(|_| "board_unknown".to_string());

    let combined = format!("{}|{}", cpu_raw, board_raw);

    // 2. Hash the combined ID using SHA-256
    let digest = digest::digest(&digest::SHA256, combined.as_bytes());
    hex::encode(digest)
}

/// Retrieve the CPU identifier for the current platform.
fn get_cpu_id() -> LicenseResult<String> {
    #[cfg(target_os = "windows")]
    {
        windows::get_cpu_id()
            .map_err(|e| LicenseError::ServerError(format!("Win CPU ID error: {e}")))
    }

    #[cfg(target_os = "macos")]
    {
        macos::get_cpu_id()
            .map_err(|e| LicenseError::ServerError(format!("macOS CPU ID error: {e}")))
    }

    #[cfg(target_os = "linux")]
    {
        linux::get_cpu_id()
            .map_err(|e| LicenseError::ServerError(format!("Linux CPU ID error: {e}")))
    }
}

/// Retrieve the motherboard / board serial identifier.
fn get_motherboard_id() -> LicenseResult<String> {
    #[cfg(target_os = "windows")]
    {
        windows::get_motherboard_id()
            .map_err(|e| LicenseError::ServerError(format!("Win Board ID error: {e}")))
    }

    #[cfg(target_os = "macos")]
    {
        macos::get_motherboard_id()
            .map_err(|e| LicenseError::ServerError(format!("macOS Board ID error: {e}")))
    }

    #[cfg(target_os = "linux")]
    {
        linux::get_motherboard_id()
            .map_err(|e| LicenseError::ServerError(format!("Linux Board ID error: {e}")))
    }
}
