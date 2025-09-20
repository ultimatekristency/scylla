use serde::Deserialize;
use config::ConfigError;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub github_owner: String,
    pub github_repo: String,
    pub binary_name: String,
    pub database_url: String, // Added for Neon
    pub update_interval_secs: u64, // Added for configurable interval
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        // Load .env file if present
        dotenvy::dotenv().ok();

        // Build config from environment variables
        config::Config::builder()
            .add_source(config::Environment::default())
            .build()?
            .try_deserialize()
    }
}