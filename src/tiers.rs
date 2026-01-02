//! Tier configuration system for Talos.
//!
//! Tiers allow you to define feature sets and limits that can be associated with licenses.
//! This is completely optional - if you don't need tiers, simply don't configure any.
//!
//! # Configuration
//!
//! Tiers are defined in your `config.toml`:
//!
//! ```toml
//! [tiers.free]
//! features = []
//! bandwidth_gb = 0
//!
//! [tiers.pro]
//! features = ["feature_a", "feature_b"]
//! bandwidth_gb = 500
//!
//! [tiers.enterprise]
//! features = ["feature_a", "feature_b", "feature_c"]
//! bandwidth_gb = 0  # 0 means unlimited
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use talos::tiers::{get_tier_config, get_tier_features, get_bandwidth_limit_bytes};
//!
//! // Get full tier configuration
//! if let Some(tier) = get_tier_config("pro") {
//!     println!("Tier: {}", tier.name);
//!     println!("Features: {:?}", tier.features);
//! }
//!
//! // Get just the features for a tier
//! let features = get_tier_features("pro");
//!
//! // Get bandwidth limit in bytes (None if unlimited or tier doesn't exist)
//! let limit = get_bandwidth_limit_bytes("pro");
//! ```

use serde::Deserialize;
use std::collections::HashMap;

use crate::config::get_config;

/// Configuration for a single tier.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TierConfig {
    /// List of features included in this tier
    pub features: Vec<String>,
    /// Bandwidth limit in gigabytes (0 = unlimited)
    pub bandwidth_gb: u64,
}

impl TierConfig {
    /// Check if this tier includes a specific feature.
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }

    /// Get the bandwidth limit in bytes.
    /// Returns None if bandwidth is unlimited (0).
    pub fn bandwidth_limit_bytes(&self) -> Option<u64> {
        if self.bandwidth_gb == 0 {
            None
        } else {
            Some(self.bandwidth_gb * 1024 * 1024 * 1024)
        }
    }
}

/// A tier with its name included (for when you need the full context).
#[derive(Debug, Clone)]
pub struct Tier {
    /// The tier name (e.g., "free", "pro", "enterprise")
    pub name: String,
    /// The tier configuration
    pub config: TierConfig,
}

impl Tier {
    /// Check if this tier includes a specific feature.
    pub fn has_feature(&self, feature: &str) -> bool {
        self.config.has_feature(feature)
    }

    /// Get the features included in this tier.
    pub fn features(&self) -> &[String] {
        &self.config.features
    }

    /// Get the bandwidth limit in bytes.
    /// Returns None if bandwidth is unlimited (0).
    pub fn bandwidth_limit_bytes(&self) -> Option<u64> {
        self.config.bandwidth_limit_bytes()
    }
}

/// Get the configuration for a specific tier.
///
/// Returns `Some(Tier)` if the tier exists, `None` otherwise.
///
/// # Example
///
/// ```rust,ignore
/// use talos::tiers::get_tier_config;
///
/// if let Some(tier) = get_tier_config("pro") {
///     println!("Pro tier has {} features", tier.config.features.len());
/// }
/// ```
pub fn get_tier_config(tier_name: &str) -> Option<Tier> {
    let config = get_config().ok()?;
    config.tiers.get(tier_name).map(|tier_config| Tier {
        name: tier_name.to_string(),
        config: tier_config.clone(),
    })
}

/// Get the features included in a tier.
///
/// Returns an empty Vec if the tier doesn't exist.
///
/// # Example
///
/// ```rust,ignore
/// use talos::tiers::get_tier_features;
///
/// let features = get_tier_features("pro");
/// for feature in features {
///     println!("Feature: {}", feature);
/// }
/// ```
pub fn get_tier_features(tier_name: &str) -> Vec<String> {
    get_tier_config(tier_name)
        .map(|t| t.config.features)
        .unwrap_or_default()
}

/// Get the bandwidth limit for a tier in bytes.
///
/// Returns `None` if:
/// - The tier doesn't exist
/// - The tier has unlimited bandwidth (bandwidth_gb = 0)
///
/// # Example
///
/// ```rust,ignore
/// use talos::tiers::get_bandwidth_limit_bytes;
///
/// match get_bandwidth_limit_bytes("pro") {
///     Some(limit) => println!("Limit: {} bytes", limit),
///     None => println!("Unlimited or tier not found"),
/// }
/// ```
pub fn get_bandwidth_limit_bytes(tier_name: &str) -> Option<u64> {
    get_tier_config(tier_name).and_then(|t| t.bandwidth_limit_bytes())
}

/// Check if a tier exists.
pub fn tier_exists(tier_name: &str) -> bool {
    get_tier_config(tier_name).is_some()
}

/// Check if a tier includes a specific feature.
///
/// Returns `false` if the tier doesn't exist.
pub fn tier_has_feature(tier_name: &str, feature: &str) -> bool {
    get_tier_config(tier_name)
        .map(|t| t.has_feature(feature))
        .unwrap_or(false)
}

/// Get all configured tier names.
pub fn get_all_tier_names() -> Vec<String> {
    get_config()
        .map(|c| c.tiers.keys().cloned().collect())
        .unwrap_or_default()
}

/// Get all configured tiers.
pub fn get_all_tiers() -> HashMap<String, TierConfig> {
    get_config().map(|c| c.tiers.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_config_has_feature() {
        let config = TierConfig {
            features: vec!["feature_a".to_string(), "feature_b".to_string()],
            bandwidth_gb: 100,
        };

        assert!(config.has_feature("feature_a"));
        assert!(config.has_feature("feature_b"));
        assert!(!config.has_feature("feature_c"));
    }

    #[test]
    fn tier_config_bandwidth_limit() {
        // Unlimited (0)
        let unlimited = TierConfig {
            features: vec![],
            bandwidth_gb: 0,
        };
        assert_eq!(unlimited.bandwidth_limit_bytes(), None);

        // Limited
        let limited = TierConfig {
            features: vec![],
            bandwidth_gb: 100, // 100 GB
        };
        assert_eq!(
            limited.bandwidth_limit_bytes(),
            Some(100 * 1024 * 1024 * 1024)
        );
    }

    #[test]
    fn tier_wrapper_delegates_correctly() {
        let tier = Tier {
            name: "test".to_string(),
            config: TierConfig {
                features: vec!["feature_a".to_string()],
                bandwidth_gb: 50,
            },
        };

        assert_eq!(tier.name, "test");
        assert!(tier.has_feature("feature_a"));
        assert!(!tier.has_feature("feature_b"));
        assert_eq!(tier.features(), &["feature_a".to_string()]);
        assert_eq!(tier.bandwidth_limit_bytes(), Some(50 * 1024 * 1024 * 1024));
    }

    #[test]
    fn empty_tier_config() {
        let config = TierConfig::default();
        assert!(config.features.is_empty());
        assert_eq!(config.bandwidth_gb, 0);
        assert_eq!(config.bandwidth_limit_bytes(), None);
        assert!(!config.has_feature("anything"));
    }
}
