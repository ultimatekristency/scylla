use std::error::Error;
use std::fs::{OpenOptions};
use std::io::Write;
use self_update::cargo_crate_version;
use crate::config::Config;
use deadpool_postgres::{Manager, Pool, ManagerConfig};
use dotenvy::dotenv;
use chrono::Utc;
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;

/// Check GitHub for updates and install if available
pub async fn run_update(cfg: &Config) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Clone the config data to move into the blocking task
    let github_owner = cfg.github_owner.clone();
    let github_repo = cfg.github_repo.clone();
    let binary_name = cfg.binary_name.clone();
    
    // Run the blocking update operation in a separate thread pool
    let result = tokio::task::spawn_blocking(move || {
        self_update::backends::github::Update::configure()
            .repo_owner(&github_owner)
            .repo_name(&github_repo)
            .bin_name(&binary_name)
            .show_download_progress(false) // Disable progress for background
            .current_version(cargo_crate_version!())
            .build()?
            .update()?;
        Ok::<(), Box<dyn Error + Send + Sync>>(())
    }).await?;
    
    result
}

/// Create a database connection pool for Neon
pub fn create_db_pool(cfg: &Config) -> Result<Pool, Box<dyn Error + Send + Sync>> {
    dotenv()?;
    let _manager_config = ManagerConfig {
        recycling_method: deadpool_postgres::RecyclingMethod::Fast,
    };
    
    // Log the database URL (first 20 chars for security)
    let url_preview = if cfg.database_url.len() > 20 {
        format!("{}...", &cfg.database_url[..20])
    } else {
        cfg.database_url.clone()
    };
    log::info!("Connecting to database: {}", url_preview);
    
    // Use native TLS for Neon database connection
    let connector = TlsConnector::new()?;
    let connector = MakeTlsConnector::new(connector);
    
    let config = cfg.database_url.parse::<tokio_postgres::Config>()?;
    let manager = Manager::new(config, connector);
    let pool = Pool::builder(manager)
        .max_size(10)
        .build()?;
    Ok(pool)
}

/// Log message to Neon database or cache locally if offline
pub async fn add_log(message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Check network availability
    if !is_network_available().await {
        // Cache log to file
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open("log_cache.txt")?;
        writeln!(file, "{}: {}", Utc::now(), message)?;
        log::info!("Cached log locally: '{}'", message);
        return Ok(());
    }

    // Use global pool (assumed to be stored in a static or passed via main)
    let pool = crate::POOL.get().ok_or("Database pool not initialized")?;
    let client = pool.get().await?;
    let inserted_rows = client
        .execute(
            "INSERT INTO \"scylla-logs\" (message, created_at) VALUES ($1, CURRENT_TIMESTAMP)",
            &[&message],
        )
        .await?;

    if inserted_rows == 0 {
        log::error!("Failed to log message to Neon: '{}'", message);
    }
    Ok(())
}

/// Sync cached logs to Neon when network is restored
pub async fn sync_cached_logs(pool: &Pool) -> Result<(), Box<dyn Error + Send + Sync>> {
    let path = "log_cache.txt";
    if !std::path::Path::new(path).exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(path)?;
    let client = pool.get().await?;
    for line in content.lines() {
        let parts: Vec<&str> = line.splitn(2, ": ").collect();
        if parts.len() == 2 {
            let message = parts[1];
            client
                .execute(
                    "INSERT INTO \"scylla-logs\" (message, created_at) VALUES ($1, $2)",
                    &[&message, &parts[0]],
                )
                .await?;
        }
    }

    // Clear cache after syncing
    std::fs::remove_file(path)?;
    log::info!("Synced and cleared cached logs");
    Ok(())
}

/// Check network availability
pub async fn is_network_available() -> bool {
    isahc::get_async("https://1.1.1.1")
        .await
        .map(|_| true)
        .unwrap_or(false)
}