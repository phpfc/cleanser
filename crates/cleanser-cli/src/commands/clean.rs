use crate::progress::CliProgress;
use crate::tui;
use cleanser_core::{
    self as core, delete_items, filter_by_risk, get_cache_age, load_scan_results,
    update_cache_after_deletion, RiskLevel, ScanResults, ScanSpeed,
};
use colored::Colorize;
use humansize::{format_size, BINARY};
use std::io;

pub fn execute(
    risk: RiskLevel,
    yes: bool,
    dry_run: bool,
    force_scan: bool,
    interactive: bool,
) -> anyhow::Result<()> {
    if dry_run {
        println!("{}", "DRY RUN MODE - No files will be deleted".yellow());
    }

    println!(
        "{}",
        format!("Cleaning with maximum risk level: {}", risk).cyan()
    );

    // If interactive mode, handle files interactively based on risk level
    if interactive {
        return handle_interactive_clean(risk, dry_run, force_scan);
    }

    if !yes && !dry_run {
        println!("{}", "This will delete files. Continue? (y/N)".yellow());
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    let progress = CliProgress::new_spinner();
    let result = core::clean_with_progress(risk, dry_run, force_scan, &progress)?;
    progress.finish();

    println!("\n{}", "=== Cleanup Summary ===".green().bold());
    println!(
        "Cleaned: {} items",
        result.cleaned_count.to_string().green().bold()
    );
    println!(
        "Failed: {} items",
        result.failed_count.to_string().red().bold()
    );
    println!(
        "Space freed: {}",
        format_size(result.cleaned_size, BINARY).green().bold()
    );

    Ok(())
}

fn handle_interactive_clean(
    risk: RiskLevel,
    dry_run: bool,
    force_scan: bool,
) -> anyhow::Result<()> {
    // Try to load from cache first, unless force_scan is specified
    let results = if !force_scan {
        match load_scan_results(None) {
            Ok(Some(cached_results)) => {
                if let Ok(Some(age)) = get_cache_age() {
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
                run_default_scan()?
            }
            Err(e) => {
                println!(
                    "{}",
                    format!("Failed to load cache ({}), running fresh scan...", e).yellow()
                );
                run_default_scan()?
            }
        }
    } else {
        println!("{}", "Running fresh scan (--force-scan)...".cyan());
        run_default_scan()?
    };

    // Filter items based on risk level
    let filtered_items = filter_by_risk(&results, risk);

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
    let filtered_results = ScanResults {
        items: filtered_items,
        total_size: 0,
        scan_speed: ScanSpeed::Normal,
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

        // Convert to CleanableItem references
        let items_to_delete: Vec<_> = selected_items
            .iter()
            .filter_map(|fi| {
                filtered_results
                    .items
                    .iter()
                    .find(|item| item.path == fi.path)
                    .cloned()
            })
            .collect();

        let result = delete_items(&items_to_delete, false)?;

        println!("\n=== Deletion Summary ===");
        println!("{}: {}", "Deleted".green(), result.cleaned_count);
        println!("{}: {}", "Failed".red(), result.failed_count);

        // Update cache to remove deleted items
        if !result.deleted_paths.is_empty() {
            if let Err(e) = update_cache_after_deletion(&result.deleted_paths) {
                eprintln!(
                    "{}",
                    format!("Warning: Failed to update cache: {}", e).yellow()
                );
            }
        }
    }

    Ok(())
}

fn run_default_scan() -> anyhow::Result<ScanResults> {
    use cleanser_core::{
        home_dir_or_err, save_scan_results, scan, IgnoreList, ScanConfig, WhitelistConfig,
    };

    let mut ignore_patterns = IgnoreList::new();
    match WhitelistConfig::load() {
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

    let config = ScanConfig {
        speed: ScanSpeed::Normal,
        paths: vec![home_dir_or_err()?],
        min_file_size_mb: 100,
        max_depth: Some(6),
        find_duplicates: false,
        ignore_patterns,
        size_range: None,
        age_criteria: None,
    };

    let results = scan(config)?;

    if let Err(e) = save_scan_results(&results) {
        eprintln!(
            "{}",
            format!("Warning: Failed to save scan cache: {}", e).yellow()
        );
    }

    Ok(results)
}
