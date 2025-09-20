mod config;
mod updater;

fn main() {
    let cfg = config::Config::from_env()
        .expect("❌ Failed to load configuration from environment variables");

    println!("✅ Loaded config: {:?}", cfg);

    if let Err(e) = updater::run_update(&cfg) {
        eprintln!("❌ Update failed: {}", e);
    }
}
