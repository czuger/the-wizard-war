//! Runtime settings read from the environment.

use std::path::PathBuf;

const DEFAULT_DATABASE_URL: &str = "sqlite://wizard_war.db";
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";

/// Server settings.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// SQLite URL (`DATABASE_URL`), the file is created if missing.
    pub database_url: String,
    /// Address the server listens on (`BIND_ADDR`).
    pub bind_addr: String,
    /// Directory holding `img/` and `static/` (`ASSET_ROOT`).
    pub asset_root: PathBuf,
}

impl AppConfig {
    /// Reads the settings from environment variables, falling back to local defaults.
    pub fn from_env() -> AppConfig {
        let var =
            |name: &str, default: &str| std::env::var(name).unwrap_or_else(|_| default.to_string());
        AppConfig {
            database_url: var("DATABASE_URL", DEFAULT_DATABASE_URL),
            bind_addr: var("BIND_ADDR", DEFAULT_BIND_ADDR),
            asset_root: PathBuf::from(var("ASSET_ROOT", ".")),
        }
    }
}
