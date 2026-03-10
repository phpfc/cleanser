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

        /// Securely delete files by overwriting data before removal
        #[arg(long)]
        secure: bool,

        /// Number of secure delete passes (1, 3, 7, or 35 for Gutmann)
        #[arg(long, default_value = "3")]
        secure_passes: u8,

        /// Move files to trash instead of permanent deletion
        #[arg(long)]
        trash: bool,
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
    /// Manage trash/recycle bin
    Trash {
        #[command(subcommand)]
        action: TrashAction,
    },
    /// Manage scheduled cleanup jobs
    Schedule {
        #[command(subcommand)]
        action: ScheduleAction,
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

#[derive(Subcommand)]
enum TrashAction {
    /// List items in trash
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Restore item from trash
    Restore {
        /// Entry ID (or partial ID)
        entry: String,
        /// Restore to different location
        #[arg(long)]
        to: Option<PathBuf>,
    },
    /// Permanently delete item from trash
    Delete {
        /// Entry ID (or partial ID)
        entry: String,
        /// Use secure delete
        #[arg(long)]
        secure: bool,
    },
    /// Empty entire trash
    Empty {
        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
        /// Use secure delete for all items
        #[arg(long)]
        secure: bool,
    },
    /// Show trash statistics
    Stats,
}

#[derive(Subcommand)]
enum ScheduleAction {
    /// Create a new scheduled job
    Set {
        /// Job name
        name: String,

        /// Schedule frequency (hourly, daily@09:00, weekly@Mon,Wed,Fri@14:30, etc.)
        #[arg(short, long)]
        frequency: String,

        /// Maximum risk level (safe, moderate, risky)
        #[arg(short, long, default_value = "safe")]
        risk: RiskLevelArg,

        /// Paths to scan (defaults to home)
        #[arg(long)]
        paths: Vec<PathBuf>,

        /// Use trash instead of permanent delete
        #[arg(long)]
        trash: bool,

        /// Use secure delete
        #[arg(long)]
        secure: bool,

        /// Secure delete passes
        #[arg(long, default_value = "3")]
        secure_passes: u8,

        /// Send notification on completion
        #[arg(long)]
        notify: bool,
    },
    /// List all scheduled jobs
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Remove a scheduled job
    Remove {
        /// Job name or ID
        job: String,
    },
    /// Enable a scheduled job
    Enable {
        /// Job name or ID
        job: String,
    },
    /// Disable a scheduled job
    Disable {
        /// Job name or ID
        job: String,
    },
    /// Show job run history
    History {
        /// Job name (optional, shows all if not specified)
        job: Option<String>,

        /// Number of entries to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Run a job immediately
    Run {
        /// Job name or ID
        job: String,

        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
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
            secure,
            secure_passes,
            trash,
        } => commands::clean::execute(
            risk.into(),
            yes,
            dry_run,
            force_scan,
            interactive,
            secure,
            secure_passes,
            trash,
        ),
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
        Commands::Trash { action } => match action {
            TrashAction::List { json } => commands::trash::list(json),
            TrashAction::Restore { entry, to } => commands::trash::restore(entry, to),
            TrashAction::Delete { entry, secure } => commands::trash::delete(entry, secure),
            TrashAction::Empty { yes, secure } => commands::trash::empty(yes, secure),
            TrashAction::Stats => commands::trash::stats(),
        },
        Commands::Schedule { action } => match action {
            ScheduleAction::Set {
                name,
                frequency,
                risk,
                paths,
                trash,
                secure,
                secure_passes,
                notify,
            } => commands::schedule::set(
                name,
                frequency,
                risk.into(),
                paths,
                trash,
                secure,
                secure_passes,
                notify,
            ),
            ScheduleAction::List { json } => commands::schedule::list(json),
            ScheduleAction::Remove { job } => commands::schedule::remove(job),
            ScheduleAction::Enable { job } => commands::schedule::enable(job),
            ScheduleAction::Disable { job } => commands::schedule::disable(job),
            ScheduleAction::History { job, limit } => commands::schedule::history(job, limit),
            ScheduleAction::Run { job, dry_run } => commands::schedule::run(job, dry_run),
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
