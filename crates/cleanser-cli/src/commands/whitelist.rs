use cleanser_core::WhitelistConfig;
use colored::Colorize;
use std::path::PathBuf;

pub fn add(path: PathBuf) -> anyhow::Result<()> {
    let mut config = WhitelistConfig::load()?;
    config.add_path(path.clone())?;
    println!(
        "{}",
        format!("Added '{}' to whitelist", path.display()).green()
    );
    Ok(())
}

pub fn remove(path: PathBuf) -> anyhow::Result<()> {
    let mut config = WhitelistConfig::load()?;
    if config.remove_path(&path)? {
        println!(
            "{}",
            format!("Removed '{}' from whitelist", path.display()).green()
        );
    } else {
        println!(
            "{}",
            format!("Path '{}' not found in whitelist", path.display()).yellow()
        );
    }
    Ok(())
}

pub fn list() -> anyhow::Result<()> {
    let config = WhitelistConfig::load()?;
    let paths = config.list_paths();
    if paths.is_empty() {
        println!("Whitelist is empty");
    } else {
        println!("Whitelisted paths:");
        for path in paths {
            println!("  {}", path.display());
        }
    }
    Ok(())
}
