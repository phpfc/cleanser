mod cache;
mod cleaner;
mod scanner;
mod tui;
mod types;

use clap::{Parser, Subcommand};
use colored::Colorize;
use humansize::{format_size, BINARY};
use std::path::PathBuf;
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
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
        } => {
            println!("{}", format!("Scanning with {} speed...", speed).cyan());

            // Build ignore list from command line arguments
            let mut ignore_patterns = types::IgnoreList::new();
            
            // Load whitelist and add to ignore patterns
            match types::WhitelistConfig::load() {
                Ok(whitelist) => {
                    for path in whitelist.list_paths() {
                        if let Err(e) = ignore_patterns.add_pattern(&path.to_string_lossy()) {
                            eprintln!("{}", format!("Warning: Could not add whitelisted path '{}': {}", path.display(), e).yellow());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}", format!("Warning: Could not load whitelist: {}", e).yellow());
                }
            }
            
            // Add command-line ignore patterns
            for ignore_path in ignore {
                match ignore_patterns.add_pattern(&ignore_path) {
                    Ok(_) => {},
                    Err(e) => {
                        eprintln!("{}", format!("Warning: Could not add ignore pattern '{}': {}", ignore_path, e).yellow());
                    }
                }
            }

            // Parse size range if provided
            let parsed_size_range = if let Some(range_str) = size_range {
                match types::SizeRange::parse(&range_str) {
                    Ok(range) => Some(range),
                    Err(e) => {
                        eprintln!("{}", format!("Error: Invalid size range '{}': {}", range_str, e).red());
                        return Err(e);
                    }
                }
            } else {
                None
            };

            // Parse age criteria if provided
            let parsed_age_criteria = if older_than.is_some() || newer_than.is_some() {
                let mut criteria = types::AgeCriteria::new();
                
                if let Some(older_str) = older_than {
                    match types::parse_duration(&older_str) {
                        Ok(duration) => criteria.set_older_than(duration),
                        Err(e) => {
                            eprintln!("{}", format!("Error: Invalid duration '{}': {}", older_str, e).red());
                            return Err(e);
                        }
                    }
                }
                
                if let Some(newer_str) = newer_than {
                    match types::parse_duration(&newer_str) {
                        Ok(duration) => criteria.set_newer_than(duration),
                        Err(e) => {
                            eprintln!("{}", format!("Error: Invalid duration '{}': {}", newer_str, e).red());
                            return Err(e);
                        }
                    }
                }
                
                Some(criteria)
            } else {
                None
            };

            let config = types::ScanConfig {
                speed,
                paths: if paths.is_empty() {
                    vec![PathBuf::from(std::env::var("HOME")?)]
                } else {
                    paths.into_iter().map(PathBuf::from).collect()
                },
                min_file_size_mb: min_size,
                max_depth,
                find_duplicates,
                ignore_patterns,
                size_range: parsed_size_range,
                age_criteria: parsed_age_criteria,
                interactive_mode: interactive,
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

            // If interactive mode, launch TUI
            if interactive {
                let selected_items = tui::run_interactive_mode(&results)?;

                if selected_items.is_empty() {
                    println!("No items selected for deletion");
                } else {
                    println!("\n{}", format!("Deleting {} selected items...", selected_items.len()).cyan());

                    let mut deleted_count = 0;
                    let mut failed_count = 0;
                    let mut deleted_paths: Vec<PathBuf> = Vec::new();

                    for item in &selected_items {
                        match std::fs::remove_dir_all(&item.path).or_else(|_| std::fs::remove_file(&item.path)) {
                            Ok(_) => {
                                println!("{}", format!("✓ Deleted: {}", item.path.display()).green());
                                deleted_count += 1;
                                deleted_paths.push(item.path.clone());
                            }
                            Err(e) => {
                                eprintln!("{}", format!("✗ Failed to delete {}: {}", item.path.display(), e).red());
                                failed_count += 1;
                            }
                        }
                    }

                    println!("\n=== Deletion Summary ===");
                    println!("Deleted: {} items", deleted_count);
                    println!("Failed: {} items", failed_count);

                    // Update cache to remove deleted items
                    if !deleted_paths.is_empty() && !no_cache {
                        if let Err(e) = cache::update_cache_after_deletion(&deleted_paths) {
                            eprintln!(
                                "{}",
                                format!("Warning: Failed to update cache: {}", e).yellow()
                            );
                        }
                    }
                }

                return Ok(());
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
            interactive,
        } => {
            if dry_run {
                println!("{}", "DRY RUN MODE - No files will be deleted".yellow());
            }

            println!(
                "{}",
                format!("Cleaning with maximum risk level: {}", risk).cyan()
            );

            // If interactive mode, handle files interactively based on risk level
            if interactive {
                // Try to load from cache first, unless force_scan is specified
                let results = if !force_scan {
                    match cache::load_scan_results(None) {
                        Ok(Some(cached_results)) => {
                            if let Ok(Some(age)) = cache::get_cache_age() {
                                let mins = age / 60;
                                let secs = age % 60;
                                if mins > 0 {
                                    println!(
                                        "{}",
                                        format!(
                                            "Using cached scan results from {} min {} sec ago",
                                            mins, secs
                                        )
                                        .cyan()
                                    );
                                } else {
                                    println!(
                                        "{}",
                                        format!("Using cached scan results from {} seconds ago", secs).cyan()
                                    );
                                }
                                println!("{}", "Tip: Use --force-scan to run a fresh scan".dimmed());
                            }
                            cached_results
                        }
                        Ok(None) => {
                            println!("{}", "No cached scan found, running fresh scan...".cyan());

                            // Load whitelist
                            let mut ignore_patterns = types::IgnoreList::new();
                            match types::WhitelistConfig::load() {
                                Ok(whitelist) => {
                                    for path in whitelist.list_paths() {
                                        if let Err(e) = ignore_patterns.add_pattern(&path.to_string_lossy()) {
                                            eprintln!("{}", format!("Warning: Could not add whitelisted path '{}': {}", path.display(), e).yellow());
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("{}", format!("Warning: Could not load whitelist: {}", e).yellow());
                                }
                            }

                            let config = types::ScanConfig {
                                speed: types::ScanSpeed::Normal,
                                paths: vec![PathBuf::from(std::env::var("HOME")?)],
                                min_file_size_mb: 100,
                                max_depth: Some(6),
                                find_duplicates: false,
                                ignore_patterns,
                                size_range: None,
                                age_criteria: None,
                                interactive_mode: false,
                            };

                            let scan_results = scanner::scan(config)?;

                            // Save to cache
                            if let Err(e) = cache::save_scan_results(&scan_results) {
                                eprintln!(
                                    "{}",
                                    format!("Warning: Failed to save scan cache: {}", e).yellow()
                                );
                            }

                            scan_results
                        }
                        Err(e) => {
                            println!(
                                "{}",
                                format!("Failed to load cache ({}), running fresh scan...", e).yellow()
                            );

                            // Load whitelist
                            let mut ignore_patterns = types::IgnoreList::new();
                            match types::WhitelistConfig::load() {
                                Ok(whitelist) => {
                                    for path in whitelist.list_paths() {
                                        if let Err(e) = ignore_patterns.add_pattern(&path.to_string_lossy()) {
                                            eprintln!("{}", format!("Warning: Could not add whitelisted path '{}': {}", path.display(), e).yellow());
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("{}", format!("Warning: Could not load whitelist: {}", e).yellow());
                                }
                            }

                            let config = types::ScanConfig {
                                speed: types::ScanSpeed::Normal,
                                paths: vec![PathBuf::from(std::env::var("HOME")?)],
                                min_file_size_mb: 100,
                                max_depth: Some(6),
                                find_duplicates: false,
                                ignore_patterns,
                                size_range: None,
                                age_criteria: None,
                                interactive_mode: false,
                            };

                            let scan_results = scanner::scan(config)?;

                            // Save to cache
                            if let Err(e) = cache::save_scan_results(&scan_results) {
                                eprintln!(
                                    "{}",
                                    format!("Warning: Failed to save scan cache: {}", e).yellow()
                                );
                            }

                            scan_results
                        }
                    }
                } else {
                    println!("{}", "Running fresh scan (--force-scan)...".cyan());

                    // Load whitelist
                    let mut ignore_patterns = types::IgnoreList::new();
                    match types::WhitelistConfig::load() {
                        Ok(whitelist) => {
                            for path in whitelist.list_paths() {
                                if let Err(e) = ignore_patterns.add_pattern(&path.to_string_lossy()) {
                                    eprintln!("{}", format!("Warning: Could not add whitelisted path '{}': {}", path.display(), e).yellow());
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("{}", format!("Warning: Could not load whitelist: {}", e).yellow());
                        }
                    }

                    let config = types::ScanConfig {
                        speed: types::ScanSpeed::Normal,
                        paths: vec![PathBuf::from(std::env::var("HOME")?)],
                        min_file_size_mb: 100,
                        max_depth: Some(6),
                        find_duplicates: false,
                        ignore_patterns,
                        size_range: None,
                        age_criteria: None,
                        interactive_mode: false,
                    };

                    let scan_results = scanner::scan(config)?;

                    // Save to cache
                    if let Err(e) = cache::save_scan_results(&scan_results) {
                        eprintln!(
                            "{}",
                            format!("Warning: Failed to save scan cache: {}", e).yellow()
                        );
                    }

                    scan_results
                };

                // Filter items based on risk level
                let filtered_items: Vec<_> = results.items.iter()
                    .filter(|item| {
                        match risk {
                            types::RiskLevel::Safe => item.risk_level == types::RiskLevel::Safe,
                            types::RiskLevel::Moderate => {
                                item.risk_level == types::RiskLevel::Safe ||
                                item.risk_level == types::RiskLevel::Moderate
                            },
                            types::RiskLevel::Risky => true, // All risk levels
                        }
                    })
                    .cloned()
                    .collect();

                if filtered_items.is_empty() {
                    println!("No files found matching the specified risk level");
                    return Ok(());
                }

                println!("\nFound {} files matching risk level: {}", filtered_items.len(), risk);

                // Create a ScanResults with filtered items for the TUI
                let filtered_results = types::ScanResults {
                    items: filtered_items,
                    total_size: 0, // Will be calculated by TUI
                    scan_speed: types::ScanSpeed::Normal,
                    excluded_dirs_count: 0,
                    filtered_by_size_count: 0,
                    filtered_by_age_count: 0,
                };

                // Use the full TUI interface
                let selected_items = tui::run_interactive_mode(&filtered_results)?;

                if selected_items.is_empty() {
                    println!("No items selected for deletion");
                } else {
                    if dry_run {
                        println!("\n{}", "DRY RUN MODE - No files will be deleted".yellow());
                        println!("Would delete {} selected items", selected_items.len());
                        return Ok(());
                    }

                    println!("\n{}", format!("Deleting {} selected items...", selected_items.len()).cyan());

                    let mut deleted_count = 0;
                    let mut failed_count = 0;
                    let mut deleted_paths: Vec<PathBuf> = Vec::new();

                    for item in &selected_items {
                        let result = if item.path.is_dir() {
                            std::fs::remove_dir_all(&item.path)
                        } else {
                            std::fs::remove_file(&item.path)
                        };

                        match result {
                            Ok(_) => {
                                println!("{}", format!("✓ Deleted: {}", item.path.display()).green());
                                deleted_count += 1;
                                deleted_paths.push(item.path.clone());
                            }
                            Err(e) => {
                                eprintln!("{}", format!("✗ Failed to delete {}: {}", item.path.display(), e).red());
                                failed_count += 1;
                            }
                        }
                    }

                    println!("\n=== Deletion Summary ===");
                    println!("{}: {}", "Deleted".green(), deleted_count);
                    println!("{}: {}", "Failed".red(), failed_count);

                    // Update cache to remove deleted items
                    if !deleted_paths.is_empty() {
                        if let Err(e) = cache::update_cache_after_deletion(&deleted_paths) {
                            eprintln!(
                                "{}",
                                format!("Warning: Failed to update cache: {}", e).yellow()
                            );
                        }
                    }
                }

                return Ok(());
            }

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
        Commands::Whitelist { action } => {
            let mut config = types::WhitelistConfig::load()?;
            
            match action {
                WhitelistAction::Add { path } => {
                    config.add_path(path.clone())?;
                    println!("{}", format!("Added '{}' to whitelist", path.display()).green());
                }
                WhitelistAction::Remove { path } => {
                    if config.remove_path(&path)? {
                        println!("{}", format!("Removed '{}' from whitelist", path.display()).green());
                    } else {
                        println!("{}", format!("Path '{}' not found in whitelist", path.display()).yellow());
                    }
                }
                WhitelistAction::List => {
                    let paths = config.list_paths();
                    if paths.is_empty() {
                        println!("Whitelist is empty");
                    } else {
                        println!("Whitelisted paths:");
                        for path in paths {
                            println!("  {}", path.display());
                        }
                    }
                }
            }
        }
        Commands::Cache { action } => {
            match action {
                CacheAction::Clear => {
                    match cache::clear_cache() {
                        Ok(_) => println!("{}", "Cache cleared successfully".green()),
                        Err(e) => println!("{}", format!("Failed to clear cache: {}", e).red()),
                    }
                }
                CacheAction::Show => {
                    match cache::get_cache_age() {
                        Ok(Some(age)) => {
                            let mins = age / 60;
                            let secs = age % 60;
                            if mins > 0 {
                                println!("Cache age: {} min {} sec", mins, secs);
                            } else {
                                println!("Cache age: {} seconds", secs);
                            }

                            if let Ok(Some(results)) = cache::load_scan_results(None) {
                                let total_size: u64 = results.items.iter().map(|i| i.size).sum();
                                println!("Cached items: {}", results.items.len());
                                println!("Total size: {}", format_size(total_size, BINARY));
                            }
                        }
                        Ok(None) => println!("{}", "No cache found".yellow()),
                        Err(e) => println!("{}", format!("Error reading cache: {}", e).red()),
                    }
                }
            }
        }
    }

    Ok(())
}
