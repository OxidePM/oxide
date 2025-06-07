use std::env;

pub const STORE_DIR: &'static str = "/var/lib/oxide/store";
pub const LOG_DIR: &'static str = "/var/log/oxide";
pub const STATE_DIR: &'static str = "/var/lib/oxide/var";

pub struct Config {
    pub store_dir: String,
    pub log_dir: String,
    pub state_dir: String,
}

impl Config {
    pub fn new() -> Self {
        let store_dir = env::var("OXIDE_STORE_DIR").unwrap_or(STORE_DIR.to_string());
        let log_dir = env::var("OXIDE_LOG_DIR").unwrap_or(LOG_DIR.to_string());
        let state_dir = env::var("OXIDE_STATE_DIR").unwrap_or(STATE_DIR.to_string());
        Self {
            store_dir,
            log_dir,
            state_dir,
        }
    }
}
