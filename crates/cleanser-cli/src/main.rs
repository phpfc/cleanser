mod commands;
mod config;
mod logging;
mod output;
mod progress;
mod setup;
mod tui;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cleanser")]
#[command(about = "A fast cross-platform CLI tool for clearing storage space", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan for cleanable files and directories
    Scan {
        /// Speed of the scan (quick/normal/thorough)
        #[arg(short, long, default_value = "normal")]
        speed: ScanSpeedArg,

        /// Paths to scan (defaults to home directory)
        #[arg(short, long)]
        paths: Vec<String>,

        /// Directories to exclude from scanning
        #[arg(long)]
        ignore: Vec<String>,

        /// Minimum file size in MB for large file detection
        #[arg(long, default_value = "100")]
        min_size: u64,

        /// Size range filter (e.g., 100MB-500MB, 100MB-, -500MB)
        #[arg(long)]
        size_range: Option<String>,

        /// Only show files older than duration (e.g., 90d, 2w, 6m, 1y)
        #[arg(long)]
        older_than: Option<String>,

        /// Only show files newer than duration (e.g., 7d, 1w)
        #[arg(long)]
        newer_than: Option<String>,

        /// Maximum depth for directory traversal
        #[arg(long)]
        max_depth: Option<usize>,

        /// Find duplicate files
        #[arg(long)]
        find_duplicates: bool,

        /// Interactive mode - navigate and select files to delete
        #[arg(long)]
        interactive: bool,

        /// Output results as JSON
        #[arg(long)]
        json: bool,

        /// Don't save scan results to cache
        #[arg(long)]
        no_cache: bool,
    },
    /// Clean files based on risk level
    Clean {
        /// Maximum risk level to clean (safe/moderate/risky)
        #[arg(short, long, default_value = "safe")]
        risk: RiskLevelArg,

        /// Skip confirmation prompts
        #[arg(short = 'y', long)]
        yes: bool,

        /// Dry run - show what would be deleted without deleting
        #[arg(long)]
        dry_run: bool,

        /// Force a fresh scan instead of using cached results
        #[arg(long)]
        force_scan: bool,

        /// Interactive mode - prompt for each large file
        #[arg(long)]
        interactive: bool,
    },
    /// Manage whitelist of directories to never scan or clean
    Whitelist {
        #[command(subcommand)]
        action: WhitelistAction,
    },
    /// Manage scan result cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
    /// Manage filesystem map
    Map {
        #[command(subcommand)]
        action: MapAction,
    },
    /// Manage CLI configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum WhitelistAction {
    /// Add a path to the whitelist
    Add {
        /// Path to add to whitelist
        path: PathBuf,
    },
    /// Remove a path from the whitelist
    Remove {
        /// Path to remove from whitelist
        path: PathBuf,
    },
    /// List all whitelisted paths
    List,
}

#[derive(Subcommand)]
enum CacheAction {
    /// Clear the scan cache
    Clear,
    /// Show cache information
    Show,
}

#[derive(Subcommand)]
enum MapAction {
    /// Show the current filesystem map
    Show,
    /// Rebuild the filesystem map
    Rebuild {
        /// Maximum depth for scanning
        #[arg(long, default_value = "10")]
        max_depth: usize,

        /// Minimum confidence level (0.0-1.0)
        #[arg(long, default_value = "0.6")]
        min_confidence: f32,
    },
    /// Show statistics about the filesystem map
    Stats,
    /// Verify the filesystem map (check if paths still exist)
    Verify,
    /// Suggest whitelist entries based on the map
    Suggest,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// Value to set
        value: String,
    },
    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,
    },
    /// List all configuration values
    List,
}

// Clap-compatible wrappers for core types
#[derive(Debug, Clone, Copy, ValueEnum)]
enum ScanSpeedArg {
    Quick,
    Normal,
    Thorough,
}

impl From<ScanSpeedArg> for cleanser_core::ScanSpeed {
    fn from(arg: ScanSpeedArg) -> Self {
        match arg {
            ScanSpeedArg::Quick => cleanser_core::ScanSpeed::Quick,
            ScanSpeedArg::Normal => cleanser_core::ScanSpeed::Normal,
            ScanSpeedArg::Thorough => cleanser_core::ScanSpeed::Thorough,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum RiskLevelArg {
    Safe,
    Moderate,
    Risky,
}

impl From<RiskLevelArg> for cleanser_core::RiskLevel {
    fn from(arg: RiskLevelArg) -> Self {
        match arg {
            RiskLevelArg::Safe => cleanser_core::RiskLevel::Safe,
            RiskLevelArg::Moderate => cleanser_core::RiskLevel::Moderate,
            RiskLevelArg::Risky => cleanser_core::RiskLevel::Risky,
        }
    }
}

use std::sync::mpsc;

/// Start update check in a background thread
fn start_update_check() -> Option<mpsc::Receiver<cleanser_core::VersionInfo>> {
    let cli_config = config::CliConfig::load().unwrap_or_default();

    if !cli_config.should_check_updates() {
        return None;
    }

    let (tx, rx) = mpsc::channel();

    cleanser_core::check_for_updates_background(move |result| {
        if let Ok(info) = result {
            let _ = tx.send(info);
        }
    });

    Some(rx)
}

fn main() -> anyhow::Result<()> {
    // Initialize logging
    logging::init();

    // Check for first-time setup
    if setup::is_first_run() {
        setup::run_first_time_setup()?;
    }

    // Start update check in background
    let update_receiver = start_update_check();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Scan {
            speed,
            paths,
            ignore,
            min_size,
            size_range,
            older_than,
            newer_than,
            max_depth,
            find_duplicates,
            interactive,
            json,
            no_cache,
        } => commands::scan::execute(
            speed.into(),
            paths,
            ignore,
            min_size,
            size_range,
            older_than,
            newer_than,
            max_depth,
            find_duplicates,
            interactive,
            json,
            no_cache,
        ),
        Commands::Clean {
            risk,
            yes,
            dry_run,
            force_scan,
            interactive,
        } => commands::clean::execute(risk.into(), yes, dry_run, force_scan, interactive),
        Commands::Whitelist { action } => match action {
            WhitelistAction::Add { path } => commands::whitelist::add(path),
            WhitelistAction::Remove { path } => commands::whitelist::remove(path),
            WhitelistAction::List => commands::whitelist::list(),
        },
        Commands::Cache { action } => match action {
            CacheAction::Clear => commands::cache::clear(),
            CacheAction::Show => commands::cache::show(),
        },
        Commands::Map { action } => match action {
            MapAction::Show => commands::map::show(),
            MapAction::Rebuild {
                max_depth,
                min_confidence,
            } => commands::map::rebuild(max_depth, min_confidence),
            MapAction::Stats => commands::map::stats(),
            MapAction::Verify => commands::map::verify(),
            MapAction::Suggest => commands::map::suggest(),
        },
        Commands::Config { action } => match action {
            ConfigAction::Set { key, value } => commands::config::set(&key, &value),
            ConfigAction::Get { key } => commands::config::get(&key),
            ConfigAction::List => commands::config::list(),
        },
    };

    // Check if update info is available (non-blocking)
    if let Some(receiver) = update_receiver {
        if let Ok(info) = receiver.try_recv() {
            if info.update_available {
                use colored::Colorize;
                eprintln!();
                eprintln!(
                    "{} New version available: {} (you have {})",
                    "[info]".cyan(),
                    info.latest.as_deref().unwrap_or("unknown").green(),
                    info.current.yellow()
                );
                eprintln!(
                    "{} Update with: {}",
                    "[info]".cyan(),
                    "brew upgrade cleanser".bold()
                );
            }
        }
    }

    result
}
