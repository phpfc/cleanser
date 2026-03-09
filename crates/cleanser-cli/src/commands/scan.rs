use crate::output;
use crate::progress::CliProgress;
use crate::tui;
use cleanser_core::{
    self as core, delete_items, home_dir_or_err, parse_duration, save_scan_results,
    update_cache_after_deletion, AgeCriteria, IgnoreList, ScanConfig, ScanSpeed, SizeRange,
    WhitelistConfig,
};
use colored::Colorize;
use std::path::PathBuf;

#[allow(clippy::too_many_arguments)]
pub fn execute(
    speed: ScanSpeed,
    paths: Vec<String>,
    ignore: Vec<String>,
    min_size: u64,
    size_range: Option<String>,
    older_than: Option<String>,
    newer_than: Option<String>,
    max_depth: Option<usize>,
    find_duplicates: bool,
    interactive: bool,
    json: bool,
    no_cache: bool,
) -> anyhow::Result<()> {
    println!("{}", format!("Scanning with {} speed...", speed).cyan());

    // Build ignore list from command line arguments
    let mut ignore_patterns = IgnoreList::new();

    // Load whitelist and add to ignore patterns
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
        match SizeRange::parse(&range_str) {
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
        let mut criteria = AgeCriteria::new();

        if let Some(older_str) = older_than {
            match parse_duration(&older_str) {
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
            match parse_duration(&newer_str) {
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

    let config = ScanConfig {
        speed,
        paths: if paths.is_empty() {
            vec![home_dir_or_err()?]
        } else {
            paths.into_iter().map(PathBuf::from).collect()
        },
        min_file_size_mb: min_size,
        max_depth,
        find_duplicates,
        ignore_patterns,
        size_range: parsed_size_range,
        age_criteria: parsed_age_criteria,
    };

    let progress = CliProgress::new_spinner();
    let results = core::scan_with_progress(config, &progress)?;
    progress.finish();

    // Save to cache unless --no-cache is specified
    if !no_cache {
        if let Err(e) = save_scan_results(&results) {
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

            // Convert to CleanableItem references
            let items_to_delete: Vec<_> = selected_items
                .iter()
                .filter_map(|fi| {
                    results
                        .items
                        .iter()
                        .find(|item| item.path == fi.path)
                        .cloned()
                })
                .collect();

            let result = delete_items(&items_to_delete, false)?;

            println!("\n=== Deletion Summary ===");
            println!("Deleted: {} items", result.cleaned_count);
            println!("Failed: {} items", result.failed_count);

            // Update cache to remove deleted items
            if !result.deleted_paths.is_empty() && !no_cache {
                if let Err(e) = update_cache_after_deletion(&result.deleted_paths) {
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
        output::display_scan_results(&results);
    }

    Ok(())
}
