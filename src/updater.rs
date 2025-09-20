use std::error::Error;
use self_update::cargo_crate_version;
use crate::config::Config;

/// Check GitHub for updates and install if available
pub fn run_update(cfg: &Config) -> Result<(), Box<dyn Error>> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner(&cfg.github_owner)
        .repo_name(&cfg.github_repo)
        .bin_name(&cfg.binary_name)
        .show_download_progress(true)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;

    println!(
        "Updated `{}` from {old} → {new}",
        cfg.binary_name,
        old = cargo_crate_version!(),
        new = status.version()
    );

    Ok(())
}
