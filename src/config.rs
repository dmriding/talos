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
    let config = Config::builder()
        .add_source(config::File::with_name("config").required(false))
        .build();

    if let Ok(settings) = config {
        if let Ok(url) = settings.get_string("server_url") {
            return url;
        }
    }

    // Step 3: Fallback to the server URL from the license metadata
    license.server_url.clone()
}
