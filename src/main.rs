#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod updater;

use log::{error, info};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::time;
use updater::{add_log, create_db_pool, run_update, sync_cached_logs};
use backoff::{ExponentialBackoff, future::retry};

// Global database pool
pub static POOL: OnceLock<deadpool_postgres::Pool> = OnceLock::new();

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .target(env_logger::Target::Stdout) // Change to file in production
        .init();

    info!("Starting background update service");

    // Load configuration
    let cfg = match config::Config::from_env() {
        Ok(cfg) => {
            info!("✅ Configuration loaded successfully");
            info!("Database URL: {}", if cfg.database_url.is_empty() { "NOT SET" } else { "SET" });
            cfg
        }
        Err(e) => {
            error!("❌ Failed to load configuration: {}", e);
            error!("Make sure you have a .env file with required environment variables");
            return;
        }
    };

    // Initialize Neon database pool
    match create_db_pool(&cfg) {
        Ok(pool) => {
            let _ = POOL.set(pool);
            info!("✅ Successfully initialized Neon database pool");
        }
        Err(e) => {
            error!("❌ Failed to initialize Neon database pool: {}", e);
            return;
        }
    }

    // Run update loop
    let mut interval = time::interval(Duration::from_secs(cfg.update_interval_secs));
    let mut was_offline = false;

    loop {
        interval.tick().await;

        // Check network availability
        if !updater::is_network_available().await {
            if !was_offline {
                error!("No network access, caching logs locally");
                was_offline = true;
            }
            continue;
        }

        // Network restored, sync cached logs
        if was_offline {
            if let Some(pool) = POOL.get() {
                if let Err(e) = sync_cached_logs(pool).await {
                    error!("Failed to sync cached logs: {}", e);
                }
            }
            was_offline = false;
        }

        update(&cfg).await;
    }
}

async fn update(cfg: &config::Config) {
    let backoff = ExponentialBackoff::default();

    // Attempt update with retries
    let update_result = retry(backoff, || async {
        run_update(cfg).await.map_err(|e| backoff::Error::transient(e))
    }).await;

    match update_result {
        Ok(()) => {
            let message = format!(
                "✅ Updated '{}' from {} to latest version",
                cfg.binary_name,
                self_update::cargo_crate_version!()
            );
            if let Err(e) = add_log(&message).await {
                error!("❌ Failed to log update success: {}", e);
            }
        }
        Err(e) => {
            let message = format!("❌ Update failed: {}", e);
            error!("{}", message);
            if let Err(e) = add_log(&message).await {
                error!("❌ Failed to log update failure: {}", e);
            }
        }
    }
} 