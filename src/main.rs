mod cache;
mod cleaner;
mod mapper;
mod platform;
mod scanner;
mod setup;
mod tui;
mod types;

use clap::{Parser, Subcommand};
use colored::Colorize;
use humansize::{format_size, BINARY};
use std::path::PathBuf;
use types::{RiskLevel, ScanSpeed};

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
    /// Manage filesystem map
    Map {
        #[command(subcommand)]
        action: MapAction,
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

/// Run a fresh scan with default settings for the clean command
fn run_default_scan() -> anyhow::Result<types::ScanResults> {
    let mut ignore_patterns = types::IgnoreList::new();
    match types::WhitelistConfig::load() {
        Ok(whitelist) => {
            for path in whitelist.list_paths() {
                if let Err(e) = ignore_patterns.add_pattern(&path.to_string_lossy()) {
                    eprintln!(
                        "{}",
                        format!(
                            "Warning: Could not add whitelisted path '{}': {}",
                            path.display(),
                            e
                        )
                        .yellow()
                    );
                }
            }
        }
        Err(e) => {
            eprintln!(
                "{}",
                format!("Warning: Could not load whitelist: {}", e).yellow()
            );
        }
    }

    let config = types::ScanConfig {
        speed: types::ScanSpeed::Normal,
        paths: vec![platform::home_dir_or_err()?],
        min_file_size_mb: 100,
        max_depth: Some(6),
        find_duplicates: false,
        ignore_patterns,
        size_range: None,
        age_criteria: None,
        interactive_mode: false,
    };

    let results = scanner::scan(config)?;

    if let Err(e) = cache::save_scan_results(&results) {
        eprintln!(
            "{}",
            format!("Warning: Failed to save scan cache: {}", e).yellow()
        );
    }

    Ok(results)
}

fn handle_map_command(action: MapAction) -> anyhow::Result<()> {
    use mapper::filesystem_map::DirectoryCategory;
    use mapper::{FileSystemCrawler, FileSystemMap};

    match action {
        MapAction::Show => {
            match FileSystemMap::load() {
                Ok(map) => {
                    // Header
                    println!();
                    println!(
                        "  {}",
                        "╔════════════════════════════════════════════════════════════╗".cyan()
                    );
                    println!(
                        "  {}  {:<56}  {}",
                        "║".cyan(),
                        "FILESYSTEM MAP".cyan().bold(),
                        "║".cyan()
                    );
                    println!(
                        "  {}",
                        "╚════════════════════════════════════════════════════════════╝".cyan()
                    );
                    println!();

                    // Summary stats
                    let cleanable_count = map
                        .directories
                        .values()
                        .filter(|d| {
                            matches!(
                                d.category,
                                DirectoryCategory::Ephemeral | DirectoryCategory::BuildArtifact
                            )
                        })
                        .count();

                    println!(
                        "  {} {} directories",
                        "Total mapped:".dimmed(),
                        map.total_directories
                    );
                    println!(
                        "  {} {}",
                        "Cleanable:".green().bold(),
                        format!("{} directories", cleanable_count).green()
                    );

                    let created = chrono::DateTime::from_timestamp(map.created_at as i64, 0)
                        .unwrap_or_default();
                    println!(
                        "  {} {}",
                        "Last scan:".dimmed(),
                        created.format("%Y-%m-%d %H:%M")
                    );

                    if map.is_stale() {
                        println!();
                        println!(
                            "  {}",
                            "⚠ Map is stale (>7 days). Run: cleanser map rebuild".yellow()
                        );
                    }

                    // Cleanable by category
                    println!();
                    println!(
                        "  {}",
                        "─── Cleanable Categories ──────────────────────────────────".cyan()
                    );
                    println!();

                    let mut tag_counts: Vec<_> = map.stats_by_tag().into_iter().collect();
                    tag_counts.sort_by(|a, b| b.1 .0.cmp(&a.1 .0)); // Sort by count

                    let max_count = tag_counts.first().map(|(_, (c, _))| *c).unwrap_or(1);

                    for (tag, (count, _)) in tag_counts.iter().take(10) {
                        let bar_width = ((*count * 30) / max_count).max(1);
                        let bar = "█".repeat(bar_width);

                        println!("  {:>14}  {:>5} dirs  {}", tag.yellow(), count, bar.green());
                    }

                    println!();
                    println!(
                        "  {}",
                        "─── Quick Actions ─────────────────────────────────────────".cyan()
                    );
                    println!();
                    println!(
                        "  {} cleanser scan              {}",
                        "→".green(),
                        "Scan and calculate sizes".dimmed()
                    );
                    println!(
                        "  {} cleanser map stats         {}",
                        "→".green(),
                        "View detailed breakdown".dimmed()
                    );
                    println!(
                        "  {} cleanser map rebuild       {}",
                        "→".green(),
                        "Refresh the map".dimmed()
                    );
                    println!();
                }
                Err(_) => {
                    println!();
                    println!("  {}", "No filesystem map found.".yellow());
                    println!();
                    println!("  Create one with: {}", "cleanser map rebuild".cyan());
                    println!();
                }
            }
        }
        MapAction::Rebuild {
            max_depth,
            min_confidence,
        } => {
            println!();
            println!("  {}", "Rebuilding filesystem map...".cyan());
            println!();

            let crawler = FileSystemCrawler::new()
                .with_max_depth(max_depth)
                .with_min_confidence(min_confidence)
                .with_progress(true);

            let map = crawler.crawl_full()?;
            map.save()?;

            let cleanable_count = map
                .directories
                .values()
                .filter(|d| {
                    matches!(
                        d.category,
                        DirectoryCategory::Ephemeral | DirectoryCategory::BuildArtifact
                    )
                })
                .count();

            println!();
            println!("  {}", "✓ Filesystem map rebuilt!".green().bold());
            println!();
            println!(
                "  {} {}",
                "Directories mapped:".dimmed(),
                map.total_directories
            );
            println!(
                "  {} {}",
                "Cleanable directories:".green(),
                format!("{}", cleanable_count).green().bold()
            );
            println!();
            println!(
                "  {}",
                "Run 'cleanser scan --use-map' to analyze sizes.".dimmed()
            );
            println!();
        }
        MapAction::Stats => {
            match FileSystemMap::load() {
                Ok(map) => {
                    println!();
                    println!(
                        "  {}",
                        "╔════════════════════════════════════════════════════════════╗".cyan()
                    );
                    println!(
                        "  {}  {:<56}  {}",
                        "║".cyan(),
                        "DETAILED STATISTICS".cyan().bold(),
                        "║".cyan()
                    );
                    println!(
                        "  {}",
                        "╚════════════════════════════════════════════════════════════╝".cyan()
                    );
                    println!();

                    // By Category with visual bars
                    println!(
                        "  {}",
                        "─── By Category ───────────────────────────────────────────".cyan()
                    );
                    println!();

                    // By Category
                    let stats = map.stats_by_category();
                    let mut stats_vec: Vec<_> = stats.iter().collect();
                    stats_vec.sort_by(|a, b| b.1 .0.cmp(&a.1 .0)); // Sort by count
                    let max_count = stats_vec.first().map(|(_, (c, _))| *c).unwrap_or(1);

                    for (category, (count, _)) in &stats_vec {
                        let cat_name = match category {
                            DirectoryCategory::Ephemeral => "Cache/Temp",
                            DirectoryCategory::BuildArtifact => "Build Artifacts",
                            DirectoryCategory::ApplicationData => "App Data",
                            DirectoryCategory::UserContent => "User Content",
                            DirectoryCategory::System => "System",
                            DirectoryCategory::Unknown => "Other",
                        };
                        let bar_width = ((*count * 25) / max_count).max(1);
                        let bar = "█".repeat(bar_width);

                        type ColorFn = fn(&str) -> colored::ColoredString;
                        let (cat_color, bar_color): (ColorFn, ColorFn) = match category {
                            DirectoryCategory::Ephemeral => (|s| s.green(), |s| s.green()),
                            DirectoryCategory::BuildArtifact => (|s| s.yellow(), |s| s.yellow()),
                            DirectoryCategory::UserContent => (|s| s.blue(), |s| s.blue()),
                            _ => (|s| s.white(), |s| s.white()),
                        };

                        println!(
                            "  {:>16}  {:>6} dirs  {}",
                            cat_color(cat_name),
                            count,
                            bar_color(&bar)
                        );
                    }

                    // By Tag
                    println!();
                    println!(
                        "  {}",
                        "─── By Type (Top 15) ──────────────────────────────────────".cyan()
                    );
                    println!();

                    let tag_stats = map.stats_by_tag();
                    let mut tag_vec: Vec<_> = tag_stats.iter().collect();
                    tag_vec.sort_by(|a, b| b.1 .0.cmp(&a.1 .0)); // Sort by count

                    for (tag, (count, _)) in tag_vec.iter().take(15) {
                        println!("  {:>16}  {:>6} dirs", tag.yellow(), count);
                    }

                    // Confidence breakdown
                    println!();
                    println!(
                        "  {}",
                        "─── Classification Confidence ─────────────────────────────".cyan()
                    );
                    println!();

                    let high_conf = map
                        .directories
                        .values()
                        .filter(|d| d.confidence >= 0.9)
                        .count();
                    let med_conf = map
                        .directories
                        .values()
                        .filter(|d| d.confidence >= 0.7 && d.confidence < 0.9)
                        .count();
                    let low_conf = map
                        .directories
                        .values()
                        .filter(|d| d.confidence < 0.7)
                        .count();
                    let total = map.directories.len();

                    println!(
                        "  {:>16}  {} {}",
                        "High (≥90%)".green(),
                        format!("{:>5}", high_conf).green(),
                        format!("({}%)", high_conf * 100 / total.max(1)).dimmed()
                    );
                    println!(
                        "  {:>16}  {} {}",
                        "Medium (70-90%)".yellow(),
                        format!("{:>5}", med_conf).yellow(),
                        format!("({}%)", med_conf * 100 / total.max(1)).dimmed()
                    );
                    println!(
                        "  {:>16}  {} {}",
                        "Low (<70%)".red(),
                        format!("{:>5}", low_conf).red(),
                        format!("({}%)", low_conf * 100 / total.max(1)).dimmed()
                    );

                    // Sample cleanable paths
                    println!();
                    println!(
                        "  {}",
                        "─── Sample Cleanable Paths ────────────────────────────────".cyan()
                    );
                    println!();

                    let cleanable: Vec<_> = map
                        .directories
                        .values()
                        .filter(|d| {
                            matches!(
                                d.category,
                                DirectoryCategory::Ephemeral | DirectoryCategory::BuildArtifact
                            )
                        })
                        .filter(|d| d.confidence >= 0.8)
                        .take(8)
                        .collect();

                    if cleanable.is_empty() {
                        println!(
                            "  {}",
                            "No high-confidence cleanable directories found.".dimmed()
                        );
                    } else {
                        for dir in cleanable {
                            let path_str = dir.path.to_string_lossy();
                            let display_path = if path_str.len() > 55 {
                                format!("...{}", &path_str[path_str.len() - 52..])
                            } else {
                                path_str.to_string()
                            };

                            let tag = dir.tags.first().map(|s| s.as_str()).unwrap_or("");
                            println!(
                                "  {} {:55} {}",
                                "•".green(),
                                display_path,
                                format!("[{}]", tag).dimmed()
                            );
                        }
                    }

                    println!();
                    println!("  {}", "Run 'cleanser scan' to calculate sizes.".dimmed());
                    println!();
                }
                Err(e) => {
                    println!("{}", format!("No filesystem map found: {}", e).yellow());
                    println!("{}", "Run 'cleanser map rebuild' to create one.".cyan());
                }
            }
        }
        MapAction::Verify => match FileSystemMap::load() {
            Ok(mut map) => {
                println!();
                println!("  {}", "Verifying filesystem map...".cyan());

                let total_dirs = map.directories.len();
                let invalid: Vec<_> = map
                    .directories
                    .iter()
                    .filter(|(path, _)| !path.exists())
                    .map(|(path, _)| path.clone())
                    .collect();

                if invalid.is_empty() {
                    println!("  {}", "✓ All directories still exist.".green());
                } else {
                    println!("  {} {} invalid entries", "Found".yellow(), invalid.len());

                    map.cleanup_invalid();
                    map.save()?;

                    println!(
                        "  {} Removed {} entries, {} remain.",
                        "✓".green(),
                        invalid.len(),
                        total_dirs - invalid.len()
                    );
                }
                println!();
            }
            Err(e) => {
                println!("{}", format!("No filesystem map found: {}", e).yellow());
            }
        },
        MapAction::Suggest => match FileSystemMap::load() {
            Ok(map) => {
                println!();
                println!(
                    "  {}",
                    "─── Whitelist Suggestions ─────────────────────────────────".cyan()
                );
                println!();

                let whitelist = types::WhitelistConfig::load()?;

                let suggestions: Vec<_> = map
                    .directories
                    .values()
                    .filter(|d| {
                        d.confidence >= 0.95
                            && !whitelist.contains(&d.path)
                            && matches!(
                                d.category,
                                DirectoryCategory::System | DirectoryCategory::UserContent
                            )
                    })
                    .collect();

                if suggestions.is_empty() {
                    println!(
                        "  {}",
                        "No suggestions - your whitelist looks comprehensive!".green()
                    );
                } else {
                    println!(
                        "  Found {} directories that should probably be protected:\n",
                        suggestions.len()
                    );

                    for dir in suggestions.iter().take(10) {
                        println!("  {} {}", "→".yellow(), dir.path.display());
                    }

                    println!();
                    println!("  Add with: {}", "cleanser whitelist add <path>".cyan());
                }
                println!();
            }
            Err(e) => {
                println!("{}", format!("No filesystem map found: {}", e).yellow());
            }
        },
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    // Check for first-time setup
    if setup::is_first_run() {
        setup::run_first_time_setup()?;
    }

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
                            eprintln!(
                                "{}",
                                format!(
                                    "Warning: Could not add whitelisted path '{}': {}",
                                    path.display(),
                                    e
                                )
                                .yellow()
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "{}",
                        format!("Warning: Could not load whitelist: {}", e).yellow()
                    );
                }
            }

            // Add command-line ignore patterns
            for ignore_path in ignore {
                match ignore_patterns.add_pattern(&ignore_path) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!(
                            "{}",
                            format!(
                                "Warning: Could not add ignore pattern '{}': {}",
                                ignore_path, e
                            )
                            .yellow()
                        );
                    }
                }
            }

            // Parse size range if provided
            let parsed_size_range = if let Some(range_str) = size_range {
                match types::SizeRange::parse(&range_str) {
                    Ok(range) => Some(range),
                    Err(e) => {
                        eprintln!(
                            "{}",
                            format!("Error: Invalid size range '{}': {}", range_str, e).red()
                        );
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
                            eprintln!(
                                "{}",
                                format!("Error: Invalid duration '{}': {}", older_str, e).red()
                            );
                            return Err(e);
                        }
                    }
                }

                if let Some(newer_str) = newer_than {
                    match types::parse_duration(&newer_str) {
                        Ok(duration) => criteria.set_newer_than(duration),
                        Err(e) => {
                            eprintln!(
                                "{}",
                                format!("Error: Invalid duration '{}': {}", newer_str, e).red()
                            );
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
                    vec![platform::home_dir_or_err()?]
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
                    println!(
                        "\n{}",
                        format!("Deleting {} selected items...", selected_items.len()).cyan()
                    );

                    let mut deleted_count = 0;
                    let mut failed_count = 0;
                    let mut deleted_paths: Vec<PathBuf> = Vec::new();

                    for item in &selected_items {
                        match std::fs::remove_dir_all(&item.path)
                            .or_else(|_| std::fs::remove_file(&item.path))
                        {
                            Ok(_) => {
                                println!(
                                    "{}",
                                    format!("✓ Deleted: {}", item.path.display()).green()
                                );
                                deleted_count += 1;
                                deleted_paths.push(item.path.clone());
                            }
                            Err(e) => {
                                eprintln!(
                                    "{}",
                                    format!("✗ Failed to delete {}: {}", item.path.display(), e)
                                        .red()
                                );
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
                                        format!(
                                            "Using cached scan results from {} seconds ago",
                                            secs
                                        )
                                        .cyan()
                                    );
                                }
                                println!(
                                    "{}",
                                    "Tip: Use --force-scan to run a fresh scan".dimmed()
                                );
                            }
                            cached_results
                        }
                        Ok(None) => {
                            println!("{}", "No cached scan found, running fresh scan...".cyan());
                            run_default_scan()?
                        }
                        Err(e) => {
                            println!(
                                "{}",
                                format!("Failed to load cache ({}), running fresh scan...", e)
                                    .yellow()
                            );
                            run_default_scan()?
                        }
                    }
                } else {
                    println!("{}", "Running fresh scan (--force-scan)...".cyan());
                    run_default_scan()?
                };

                // Filter items based on risk level
                let filtered_items: Vec<_> = results
                    .items
                    .iter()
                    .filter(|item| {
                        match risk {
                            types::RiskLevel::Safe => item.risk_level == types::RiskLevel::Safe,
                            types::RiskLevel::Moderate => {
                                item.risk_level == types::RiskLevel::Safe
                                    || item.risk_level == types::RiskLevel::Moderate
                            }
                            types::RiskLevel::Risky => true, // All risk levels
                        }
                    })
                    .cloned()
                    .collect();

                if filtered_items.is_empty() {
                    println!("No files found matching the specified risk level");
                    return Ok(());
                }

                println!(
                    "\nFound {} files matching risk level: {}",
                    filtered_items.len(),
                    risk
                );

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

                    println!(
                        "\n{}",
                        format!("Deleting {} selected items...", selected_items.len()).cyan()
                    );

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
                                println!(
                                    "{}",
                                    format!("✓ Deleted: {}", item.path.display()).green()
                                );
                                deleted_count += 1;
                                deleted_paths.push(item.path.clone());
                            }
                            Err(e) => {
                                eprintln!(
                                    "{}",
                                    format!("✗ Failed to delete {}: {}", item.path.display(), e)
                                        .red()
                                );
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
                    println!(
                        "{}",
                        format!("Added '{}' to whitelist", path.display()).green()
                    );
                }
                WhitelistAction::Remove { path } => {
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
        Commands::Cache { action } => match action {
            CacheAction::Clear => match cache::clear_cache() {
                Ok(_) => println!("{}", "Cache cleared successfully".green()),
                Err(e) => println!("{}", format!("Failed to clear cache: {}", e).red()),
            },
            CacheAction::Show => match cache::get_cache_age() {
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
            },
        },
        Commands::Map { action } => {
            handle_map_command(action)?;
        }
    }

    Ok(())
}
