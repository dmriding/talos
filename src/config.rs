use serde::Deserialize;
use config::Config;
use std::env;
use crate::client::License;

#[derive(Deserialize)]
struct ConfigFile {
    server_url: Option<String>,
}

/// Function to get the server URL based on priority:
/// 1. Environment variable
/// 2. Configuration file
/// 3. License metadata
pub fn get_server_url(license: &License) -> String {
    // 1. Check if an environment variable is set
    if let Ok(url) = env::var("SERVER_URL") {
        println!("Using server URL from environment variable.");
        return url;
    }

    // 2. Check configuration file (config.toml)
    let config = Config::builder()
        .add_source(config::File::with_name("config").required(false))
        .build();
    
    if let Ok(settings) = config {
        if let Ok(url) = settings.get_string("server_url") {
            println!("Using server URL from configuration file.");
            return url;
        }
    }

    // 3. Fallback to the server URL from the license metadata
    println!("Using server URL from license metadata.");
    license.server_url.clone()
}
