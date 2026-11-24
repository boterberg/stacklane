use std::path::PathBuf;

use anyhow::Context;

use crate::crypto::SecretBox;

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
    pub secrets: Option<SecretBox>,
    pub public_url: String,
    pub update_dir: PathBuf,
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
            public_url: std::env::var("PUBLIC_URL")
                .unwrap_or_else(|_| "http://localhost:5173".into())
                .trim_end_matches('/')
                .to_owned(),
            update_dir: std::env::var_os("UPDATE_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/var/lib/stacklane/update")),
            secrets: match std::env::var("SECRET_KEY") {
                Ok(key) if !key.trim().is_empty() => Some(SecretBox::from_hex(&key)?),
                _ => None,
            },
        })
    }
}
