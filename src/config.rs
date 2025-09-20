use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub github_owner: String,
    pub github_repo: String,
    pub binary_name: String,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        // load .env file if present
        dotenvy::dotenv().ok();

        // build config from environment variables
        config::Config::builder()
            .add_source(config::Environment::default())
            .build()?
            .try_deserialize()
    }
}
