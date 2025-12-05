use config::Config;
use std::env;

use crate::client::client::License;

/// In-memory representation of config values we care about.
///
/// All fields are optional here; the getters below apply defaults.
#[derive(Debug, Default)]
struct TalosConfig {
    server_url: Option<String>,
    heartbeat_interval: Option<u64>,
    enable_logging: Option<bool>,
}

/// Load configuration from `config.toml` (if present).
///
/// This is intentionally forgiving:
/// - If the file is missing, we just return an empty/default config.
/// - If individual keys are missing, we leave them as `None`.
fn load_config() -> TalosConfig {
    let builder = Config::builder().add_source(
        config::File::with_name("config")
            .required(false), // don't crash if config.toml is missing
    );

    let built = match builder.build() {
        Ok(cfg) => cfg,
        Err(_) => {
            // If config fails to load, just return defaults.
            return TalosConfig::default();
        }
    };

    let server_url = built.get_string("server_url").ok();

    let heartbeat_interval = built
        .get_int("heartbeat_interval")
        .ok()
        .and_then(|v| u64::try_from(v).ok());

    let enable_logging = built.get_bool("enable_logging").ok();

    TalosConfig {
        server_url,
        heartbeat_interval,
        enable_logging,
    }
}

/// Retrieve the server URL for Talos operations.
///
/// Precedence:
/// 1. `SERVER_URL` environment variable (or `TALOS_SERVER_URL` if you want to add that later)
/// 2. `server_url` from `config.toml`
/// 3. `license.server_url` as a final fallback
pub fn get_server_url(license: &License) -> String {
    // 1. Environment variable override
    if let Ok(url) = env::var("SERVER_URL") {
        return url;
    }

    // 2. Config file (config.toml)
    let cfg = load_config();
    if let Some(url) = cfg.server_url {
        return url;
    }

    // 3. Fallback to whatever is embedded in the license
    license.server_url.clone()
}

/// Retrieve the heartbeat interval in seconds.
///
/// Source:
/// - `heartbeat_interval` in `config.toml`
/// - Defaults to 60 seconds if missing/invalid.
pub fn get_heartbeat_interval() -> u64 {
    let cfg = load_config();
    cfg.heartbeat_interval.unwrap_or(60)
}

/// Check whether logging is enabled.
///
/// Source:
/// - `enable_logging` in `config.toml`
/// - Defaults to `false` if missing/invalid.
pub fn is_logging_enabled() -> bool {
    let cfg = load_config();
    cfg.enable_logging.unwrap_or(false)
}
