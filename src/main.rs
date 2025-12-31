mod cache;
mod cleaner;
mod scanner;
mod types;

use clap::{Parser, Subcommand};
use colored::Colorize;
use types::{RiskLevel, ScanSpeed};

#[derive(Parser)]
#[command(name = "cleanser")]
#[command(about = "A fast CLI tool for clearing macOS storage space", long_about = None)]
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
        speed: ScanSpeed,

        /// Paths to scan (defaults to home directory)
        #[arg(short, long)]
        paths: Vec<String>,

        /// Minimum file size in MB for large file detection
        #[arg(long, default_value = "100")]
        min_size: u64,

        /// Maximum depth for directory traversal
        #[arg(long)]
        max_depth: Option<usize>,

        /// Find duplicate files
        #[arg(long)]
        find_duplicates: bool,

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
        risk: RiskLevel,

        /// Skip confirmation prompts
        #[arg(short = 'y', long)]
        yes: bool,

        /// Dry run - show what would be deleted without deleting
        #[arg(long)]
        dry_run: bool,

        /// Force a fresh scan instead of using cached results
        #[arg(long)]
        force_scan: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan {
            speed,
            paths,
            min_size,
            max_depth,
            find_duplicates,
            json,
            no_cache,
        } => {
            println!("{}", format!("Scanning with {} speed...", speed).cyan());

            let config = types::ScanConfig {
                speed,
                paths: if paths.is_empty() {
                    vec![std::env::var("HOME")?]
                } else {
                    paths
                },
                min_file_size_mb: min_size,
                max_depth,
                find_duplicates,
            };

            let results = scanner::scan(config)?;

            // Save to cache unless --no-cache is specified
            if !no_cache {
                if let Err(e) = cache::save_scan_results(&results) {
                    eprintln!(
                        "{}",
                        format!("Warning: Failed to save scan cache: {}", e).yellow()
                    );
                }
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&results)?);
            } else {
                scanner::display_results(&results);
            }
        }
        Commands::Clean {
            risk,
            yes,
            dry_run,
            force_scan,
        } => {
            if dry_run {
                println!("{}", "DRY RUN MODE - No files will be deleted".yellow());
            }

            println!(
                "{}",
                format!("Cleaning with maximum risk level: {}", risk).cyan()
            );

            if !yes && !dry_run {
                println!("{}", "This will delete files. Continue? (y/N)".yellow());
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            cleaner::clean(risk, dry_run, force_scan)?;
        }
    }

    Ok(())
}
