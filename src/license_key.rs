//! License key generation and validation.
//!
//! This module provides functions for generating and validating human-readable license keys
//! in the format `PREFIX-XXXX-XXXX-XXXX-XXXX`.
//!
//! # Features
//!
//! - Configurable prefix (e.g., "LIC", "KERYX", "PRO")
//! - Configurable number of segments and segment length
//! - Uses cryptographically secure random number generation
//! - Excludes ambiguous characters (0, O, I, L, 1) for readability
//! - Format validation
//!
//! # Example
//!
//! ```rust,ignore
//! use talos::license_key::{generate_license_key, validate_license_key_format, LicenseKeyConfig};
//!
//! let config = LicenseKeyConfig::default();
//! let key = generate_license_key(&config);
//! assert!(validate_license_key_format(&key, &config));
//! ```

use rand::Rng;

use crate::config::{get_config, LicenseConfig};
use crate::errors::{LicenseError, LicenseResult};

/// Character set for license key generation.
/// Excludes ambiguous characters: 0, O, I, L, 1
const LICENSE_KEY_CHARSET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";

/// Configuration for license key generation.
/// This is a convenience wrapper that can be constructed from `LicenseConfig`.
#[derive(Debug, Clone)]
pub struct LicenseKeyConfig {
    /// Prefix for the license key (e.g., "LIC", "KERYX")
    pub prefix: String,
    /// Number of segments after the prefix
    pub segments: u8,
    /// Length of each segment
    pub segment_length: u8,
}

impl Default for LicenseKeyConfig {
    fn default() -> Self {
        Self {
            prefix: "LIC".to_string(),
            segments: 4,
            segment_length: 4,
        }
    }
}

impl From<&LicenseConfig> for LicenseKeyConfig {
    fn from(config: &LicenseConfig) -> Self {
        Self {
            prefix: config.key_prefix.clone(),
            segments: config.key_segments,
            segment_length: config.key_segment_length,
        }
    }
}

/// Generate a single segment of random characters.
fn generate_segment(length: u8) -> String {
    let mut rng = rand::rng();
    (0..length)
        .map(|_| {
            let idx = rng.random_range(0..LICENSE_KEY_CHARSET.len());
            LICENSE_KEY_CHARSET[idx] as char
        })
        .collect()
}

/// Generate a license key with the given configuration.
///
/// # Format
///
/// The generated key follows the format: `PREFIX-XXXX-XXXX-XXXX-XXXX`
/// where:
/// - `PREFIX` is the configured prefix
/// - Each `XXXX` is a segment of random characters
/// - The number of segments and their length are configurable
///
/// # Example
///
/// ```rust,ignore
/// use talos::license_key::{generate_license_key, LicenseKeyConfig};
///
/// let config = LicenseKeyConfig {
///     prefix: "KERYX".to_string(),
///     segments: 4,
///     segment_length: 4,
/// };
/// let key = generate_license_key(&config);
/// // Produces something like: "KERYX-A2B3-C4D5-E6F7-G8H9"
/// ```
pub fn generate_license_key(config: &LicenseKeyConfig) -> String {
    let segments: Vec<String> = (0..config.segments)
        .map(|_| generate_segment(config.segment_length))
        .collect();

    format!("{}-{}", config.prefix, segments.join("-"))
}

/// Generate a license key using the global configuration.
///
/// This is a convenience function that uses the configuration from `config.toml`
/// or environment variables.
///
/// # Errors
///
/// Returns an error if the configuration cannot be loaded.
pub fn generate_license_key_from_config() -> LicenseResult<String> {
    let config = get_config()?;
    let key_config = LicenseKeyConfig::from(&config.license);
    Ok(generate_license_key(&key_config))
}

/// Validate that a license key matches the expected format.
///
/// This validates:
/// - The key starts with the expected prefix
/// - The key has the correct number of segments
/// - Each segment has the correct length
/// - All characters in segments are from the valid character set
///
/// # Example
///
/// ```rust,ignore
/// use talos::license_key::{validate_license_key_format, LicenseKeyConfig};
///
/// let config = LicenseKeyConfig::default();
/// assert!(validate_license_key_format("LIC-A2B3-C4D5-E6F7-G8H9", &config));
/// assert!(!validate_license_key_format("INVALID-KEY", &config));
/// ```
pub fn validate_license_key_format(key: &str, config: &LicenseKeyConfig) -> bool {
    // Check prefix
    if !key.starts_with(&config.prefix) {
        return false;
    }

    // Split by dashes
    let parts: Vec<&str> = key.split('-').collect();

    // Expected: prefix + N segments
    let expected_parts = 1 + config.segments as usize;
    if parts.len() != expected_parts {
        return false;
    }

    // First part must be the prefix
    if parts[0] != config.prefix {
        return false;
    }

    // Validate each segment
    for segment in &parts[1..] {
        // Check length
        if segment.len() != config.segment_length as usize {
            return false;
        }

        // Check characters are valid
        for ch in segment.chars() {
            if !LICENSE_KEY_CHARSET.contains(&(ch as u8)) {
                return false;
            }
        }
    }

    true
}

/// Validate a license key format using the global configuration.
///
/// # Errors
///
/// Returns an error if the configuration cannot be loaded.
pub fn validate_license_key_format_from_config(key: &str) -> LicenseResult<bool> {
    let config = get_config()?;
    let key_config = LicenseKeyConfig::from(&config.license);
    Ok(validate_license_key_format(key, &key_config))
}

/// Parse a license key and extract its components.
///
/// Returns `Some((prefix, segments))` if the key is valid, `None` otherwise.
pub fn parse_license_key(key: &str) -> Option<(String, Vec<String>)> {
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() < 2 {
        return None;
    }

    let prefix = parts[0].to_string();
    let segments: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

    Some((prefix, segments))
}

/// Generate a unique license key, checking against existing keys.
///
/// This function generates keys until it finds one that doesn't exist in the database.
/// It will retry up to `max_retries` times before giving up.
///
/// # Arguments
///
/// * `config` - License key configuration
/// * `exists_fn` - An async function that checks if a key already exists
/// * `max_retries` - Maximum number of generation attempts
///
/// # Errors
///
/// Returns an error if a unique key cannot be generated within the retry limit.
pub async fn generate_unique_license_key<F, Fut>(
    config: &LicenseKeyConfig,
    exists_fn: F,
    max_retries: u32,
) -> LicenseResult<String>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = LicenseResult<bool>>,
{
    for _ in 0..max_retries {
        let key = generate_license_key(config);
        if !exists_fn(key.clone()).await? {
            return Ok(key);
        }
    }

    Err(LicenseError::ServerError(format!(
        "failed to generate unique license key after {max_retries} attempts"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_key_has_correct_format() {
        let config = LicenseKeyConfig::default();
        let key = generate_license_key(&config);

        // Should start with prefix
        assert!(key.starts_with("LIC-"));

        // Should have correct number of parts
        let parts: Vec<&str> = key.split('-').collect();
        assert_eq!(parts.len(), 5); // prefix + 4 segments

        // Each segment should have correct length
        for segment in &parts[1..] {
            assert_eq!(segment.len(), 4);
        }
    }

    #[test]
    fn generate_key_uses_valid_characters() {
        let config = LicenseKeyConfig::default();
        let key = generate_license_key(&config);

        // Extract segments (skip prefix)
        let parts: Vec<&str> = key.split('-').collect();
        for segment in &parts[1..] {
            for ch in segment.chars() {
                assert!(
                    LICENSE_KEY_CHARSET.contains(&(ch as u8)),
                    "Invalid character: {}",
                    ch
                );
            }
        }
    }

    #[test]
    fn generate_key_excludes_ambiguous_characters() {
        // Generate many keys to increase chance of catching issues
        let config = LicenseKeyConfig::default();
        for _ in 0..100 {
            let key = generate_license_key(&config);

            // Extract only the generated segments (skip prefix)
            let parts: Vec<&str> = key.split('-').collect();
            for segment in &parts[1..] {
                // Check no ambiguous characters in generated segments
                assert!(!segment.contains('0'), "Segment contains '0': {}", segment);
                assert!(!segment.contains('O'), "Segment contains 'O': {}", segment);
                assert!(!segment.contains('I'), "Segment contains 'I': {}", segment);
                assert!(!segment.contains('L'), "Segment contains 'L': {}", segment);
                assert!(!segment.contains('1'), "Segment contains '1': {}", segment);
            }
        }
    }

    #[test]
    fn generate_key_with_custom_config() {
        let config = LicenseKeyConfig {
            prefix: "KERYX".to_string(),
            segments: 3,
            segment_length: 5,
        };
        let key = generate_license_key(&config);

        assert!(key.starts_with("KERYX-"));
        let parts: Vec<&str> = key.split('-').collect();
        assert_eq!(parts.len(), 4); // prefix + 3 segments
        for segment in &parts[1..] {
            assert_eq!(segment.len(), 5);
        }
    }

    #[test]
    fn validate_format_accepts_valid_key() {
        let config = LicenseKeyConfig::default();
        let key = generate_license_key(&config);
        assert!(validate_license_key_format(&key, &config));
    }

    #[test]
    fn validate_format_rejects_wrong_prefix() {
        let config = LicenseKeyConfig::default();
        assert!(!validate_license_key_format(
            "WRONG-A2B3-C4D5-E6F7-G8H9",
            &config
        ));
    }

    #[test]
    fn validate_format_rejects_wrong_segment_count() {
        let config = LicenseKeyConfig::default();
        assert!(!validate_license_key_format("LIC-A2B3-C4D5", &config)); // too few
        assert!(!validate_license_key_format(
            "LIC-A2B3-C4D5-E6F7-G8H9-J2K3",
            &config
        )); // too many
    }

    #[test]
    fn validate_format_rejects_wrong_segment_length() {
        let config = LicenseKeyConfig::default();
        assert!(!validate_license_key_format(
            "LIC-A2-C4D5-E6F7-G8H9",
            &config
        )); // too short
        assert!(!validate_license_key_format(
            "LIC-A2B3C-C4D5-E6F7-G8H9",
            &config
        )); // too long
    }

    #[test]
    fn validate_format_rejects_invalid_characters() {
        let config = LicenseKeyConfig::default();
        // Contains 'O' which is excluded
        assert!(!validate_license_key_format(
            "LIC-AOOO-C4D5-E6F7-G8H9",
            &config
        ));
        // Contains '0' which is excluded
        assert!(!validate_license_key_format(
            "LIC-A000-C4D5-E6F7-G8H9",
            &config
        ));
        // Contains lowercase
        assert!(!validate_license_key_format(
            "LIC-a2b3-C4D5-E6F7-G8H9",
            &config
        ));
    }

    #[test]
    fn parse_key_extracts_components() {
        let result = parse_license_key("LIC-A2B3-C4D5-E6F7-G8H9");
        assert!(result.is_some());

        let (prefix, segments) = result.unwrap();
        assert_eq!(prefix, "LIC");
        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0], "A2B3");
        assert_eq!(segments[1], "C4D5");
        assert_eq!(segments[2], "E6F7");
        assert_eq!(segments[3], "G8H9");
    }

    #[test]
    fn parse_key_returns_none_for_invalid() {
        assert!(parse_license_key("INVALID").is_none());
        assert!(parse_license_key("").is_none());
    }

    #[test]
    fn generated_keys_are_unique() {
        let config = LicenseKeyConfig::default();
        let mut keys = std::collections::HashSet::new();

        // Generate 1000 keys and check for collisions
        for _ in 0..1000 {
            let key = generate_license_key(&config);
            assert!(keys.insert(key.clone()), "Duplicate key generated: {}", key);
        }
    }
}
