//! Configuration management commands.

use crate::config::CliConfig;
use anyhow::Result;
use colored::Colorize;

/// Set a configuration value
pub fn set(key: &str, value: &str) -> Result<()> {
    let mut config = CliConfig::load()?;

    match key {
        "check-updates" => {
            let enabled = match value.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => true,
                "false" | "0" | "no" | "off" => false,
                _ => {
                    eprintln!(
                        "{} Invalid value for check-updates: {}",
                        "error:".red().bold(),
                        value
                    );
                    eprintln!("       Use 'true' or 'false'");
                    return Ok(());
                }
            };
            config.check_updates = enabled;
            config.save()?;
            println!(
                "{} check-updates = {}",
                "Set".green(),
                if enabled { "true" } else { "false" }
            );
        }
        _ => {
            eprintln!(
                "{} Unknown configuration key: {}",
                "error:".red().bold(),
                key
            );
            eprintln!();
            eprintln!("Available keys:");
            eprintln!("  check-updates    Enable/disable automatic update checks");
        }
    }

    Ok(())
}

/// Get a configuration value
pub fn get(key: &str) -> Result<()> {
    let config = CliConfig::load()?;

    match key {
        "check-updates" => {
            println!("{}", config.check_updates);
        }
        _ => {
            eprintln!(
                "{} Unknown configuration key: {}",
                "error:".red().bold(),
                key
            );
            eprintln!();
            eprintln!("Available keys:");
            eprintln!("  check-updates    Enable/disable automatic update checks");
        }
    }

    Ok(())
}

/// List all configuration values
pub fn list() -> Result<()> {
    let config = CliConfig::load()?;

    println!("{}", "Current configuration:".bold());
    println!();
    println!(
        "  {} = {}",
        "check-updates".cyan(),
        if config.check_updates {
            "true".green()
        } else {
            "false".red()
        }
    );
    println!();

    // Show env var status
    if std::env::var("CLEANSER_NO_UPDATE_CHECK").is_ok() {
        println!(
            "  {} CLEANSER_NO_UPDATE_CHECK is set, update checks are disabled",
            "Note:".yellow()
        );
    }

    // Show config path
    if let Ok(path) = CliConfig::config_path() {
        println!();
        println!("  Config file: {}", path.display().to_string().dimmed());
    }

    Ok(())
}
