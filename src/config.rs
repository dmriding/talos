use config::Config;
use std::env;
use crate::client::License;

pub fn get_server_url(license: &License) -> String {
    if let Ok(url) = env::var("SERVER_URL") {
        return url;
    }
    let config = Config::builder()
        .add_source(config::File::with_name("config").required(false))
        .build();
    if let Ok(settings) = config {
        if let Ok(url) = settings.get_string("server_url") {
            return url;
        }
    }
    license.server_url.clone()
}
