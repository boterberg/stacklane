use std::path::PathBuf;

use anyhow::Context;

pub const VERSION: &str = match option_env!("STACKLANE_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub cookie_secure: bool,
    pub static_dir: Option<PathBuf>,
    pub demo_mode: bool,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL is not set")?,
            bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into()),
            cookie_secure: std::env::var("COOKIE_SECURE")
                .map(|value| value == "true")
                .unwrap_or(true),
            static_dir: std::env::var_os("STATIC_DIR").map(PathBuf::from),
            demo_mode: std::env::var("DEMO_MODE").is_ok_and(|value| value == "true"),
        })
    }
}
