use config::Config;
use std::env;
use crate::client::client::License;

/// Retrieves the server URL from the environment variable, configuration file, or license metadata.
pub fn get_server_url(license: &License) -> String {
    // Step 1: Check for the SERVER_URL environment variable
    if let Ok(url) = env::var("SERVER_URL") {
        return url;
    }

    // Step 2: Check the configuration file (config.toml)
    let config = load_config();
    if let Some(url) = config.get("server_url") {
        return url.clone(); // Convert &String to String
    }
    
    // Step 3: Fallback to the server URL from the license metadata
    license.server_url.clone()
}

/// Loads the configuration from config.toml
fn load_config() -> std::collections::HashMap<String, String> {
    let mut settings = std::collections::HashMap::new();
    let config = Config::builder()
        .add_source(config::File::with_name("config").required(false))
        .build();

    if let Ok(cfg) = config {
        if let Ok(url) = cfg.get_string("server_url") {
            settings.insert("server_url".to_string(), url);
        }
        if let Ok(interval) = cfg.get_int("heartbeat_interval") {
            settings.insert("heartbeat_interval".to_string(), interval.to_string());
        }
        if let Ok(enable_logging) = cfg.get_bool("enable_logging") {
            settings.insert("enable_logging".to_string(), enable_logging.to_string());
        }
    }
    settings
}

/// Retrieves the heartbeat interval from the configuration
pub fn get_heartbeat_interval() -> u64 {
    let config = load_config();
    if let Some(interval) = config.get("heartbeat_interval") {
        interval.parse().unwrap_or(60)
    } else {
        60 // default value
    }
}

/// Checks if logging is enabled from the configuration
pub fn is_logging_enabled() -> bool {
    let config = load_config();
    config.get("enable_logging").map(|val| val == "true").unwrap_or(false)
}
