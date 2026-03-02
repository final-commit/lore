use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub repo_path: String,
    pub db_path: String,
    pub search_index_path: String,
    pub jwt_secret: String,
    pub log_level: String,
    pub cors_origins: Vec<String>,
    pub webhook_secret: Option<String>,
    pub max_upload_bytes: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            host: "0.0.0.0".to_string(),
            port: 3000,
            repo_path: "./repo".to_string(),
            db_path: "./forge.db".to_string(),
            search_index_path: "./search_index".to_string(),
            jwt_secret: "change-me-in-production-use-32-chars-min".to_string(),
            log_level: "info".to_string(),
            cors_origins: vec!["http://localhost:3001".to_string()],
            webhook_secret: None,
            max_upload_bytes: 10 * 1024 * 1024, // 10 MB
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, figment::Error> {
        Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Toml::file("forge.toml"))
            .merge(Env::prefixed("FORGE_"))
            .extract()
    }

    /// Load with extra overrides (useful for testing).
    pub fn load_with(overrides: impl figment::Provider) -> Result<Self, figment::Error> {
        Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Toml::file("forge.toml"))
            .merge(Env::prefixed("FORGE_"))
            .merge(overrides)
            .extract()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialises tests that mutate env vars to avoid data races.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.host, "0.0.0.0");
        assert_eq!(cfg.port, 3000);
        assert_eq!(cfg.log_level, "info");
        assert!(cfg.webhook_secret.is_none());
        assert!(!cfg.cors_origins.is_empty());
    }

    #[test]
    fn test_config_load() {
        let _lock = ENV_LOCK.lock().unwrap();
        // Basic load should succeed using defaults when no config file present.
        let cfg = Config::load().expect("config should load");
        assert_eq!(cfg.port, 3000);
        assert_eq!(cfg.max_upload_bytes, 10 * 1024 * 1024);
    }

    #[test]
    fn test_config_env_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        // FORGE_PORT should override the default port.
        unsafe { std::env::set_var("FORGE_PORT", "9090") };
        let cfg = Config::load().expect("config should load");
        unsafe { std::env::remove_var("FORGE_PORT") };
        assert_eq!(cfg.port, 9090);
    }
}
