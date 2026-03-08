use crate::types::*;
use crate::{cache, platform, scanner};
use anyhow::Result;
use colored::Colorize;
use humansize::{format_size, BINARY};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

fn run_fresh_scan() -> Result<ScanResults> {
    // Build ignore list with whitelist
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

    let config = ScanConfig {
        speed: ScanSpeed::Normal,
        paths: vec![platform::home_dir_or_err()?],
        min_file_size_mb: 0, // Don't scan for large files during clean
        max_depth: Some(6),
        find_duplicates: false, // Don't look for duplicates during clean
        ignore_patterns,
        size_range: None,
        age_criteria: None,
        interactive_mode: false,
    };

    let results = scanner::scan(config)?;

    // Save to cache for next time
    if let Err(e) = cache::save_scan_results(&results) {
        eprintln!(
            "{}",
            format!("Warning: Failed to save scan cache: {}", e).yellow()
        );
    }

    Ok(results)
}

pub fn clean(max_risk: RiskLevel, dry_run: bool, force_scan: bool) -> Result<()> {
    // Try to load from cache first
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
                run_fresh_scan()?
            }
            Err(e) => {
                println!(
                    "{}",
                    format!("Failed to load cache ({}), running fresh scan...", e).yellow()
                );
                run_fresh_scan()?
            }
        }
    } else {
        println!("{}", "Running fresh scan (--force-scan)...".cyan());
        run_fresh_scan()?
    };

    // Filter items by risk level
    let items_to_clean: Vec<&CleanableItem> = results
        .items
        .iter()
        .filter(|item| item.risk_level <= max_risk)
        .collect();

    if items_to_clean.is_empty() {
        println!("{}", "No items found to clean.".yellow());
        return Ok(());
    }

    // Show preview
    preview_items(&items_to_clean);

    if dry_run {
        println!("{}", "DRY RUN: No files were deleted.".yellow().bold());
        return Ok(());
    }

    // Perform the cleanup with progress bar
    let mut cleaned_size = 0u64;
    let mut cleaned_count = 0usize;
    let mut failed_count = 0usize;
    let mut deleted_paths: Vec<PathBuf> = Vec::new();

    let pb = ProgressBar::new(items_to_clean.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .expect("Failed to create progress bar template")
            .progress_chars("#>-"),
    );

    for item in items_to_clean {
        // Show current file being cleaned
        let file_name = item
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        pb.set_message(format!("Cleaning: {}", file_name));

        match delete_item(&item.path) {
            Ok(size) => {
                cleaned_size += size;
                cleaned_count += 1;
                deleted_paths.push(item.path.clone());
                pb.println(format!(
                    "{} Cleaned: {}",
                    "✓".green(),
                    item.path.display().to_string().dimmed()
                ));
            }
            Err(e) => {
                failed_count += 1;
                pb.println(format!(
                    "{} Failed to clean {}: {}",
                    "✗".red(),
                    item.path.display(),
                    e
                ));
            }
        }
        pb.inc(1);
    }

    pb.finish_with_message(format!(
        "Cleanup complete! {} items cleaned, {} failed",
        cleaned_count.to_string().green(),
        failed_count.to_string().red()
    ));

    println!("\n{}", "=== Cleanup Summary ===".green().bold());
    println!(
        "Cleaned: {} items",
        cleaned_count.to_string().green().bold()
    );
    println!("Failed: {} items", failed_count.to_string().red().bold());
    println!(
        "Space freed: {}",
        format_size(cleaned_size, BINARY).green().bold()
    );

    // Update cache to remove deleted items
    if !deleted_paths.is_empty() && !force_scan {
        if let Err(e) = cache::update_cache_after_deletion(&deleted_paths) {
            eprintln!(
                "{}",
                format!("Warning: Failed to update cache: {}", e).yellow()
            );
        }
    }

    Ok(())
}

fn delete_item(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }

    // Calculate size before deletion
    let size = if path.is_dir() {
        get_dir_size_fast(path)?
    } else {
        fs::metadata(path)?.len()
    };

    // Delete the item
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }

    Ok(size)
}

fn get_dir_size_fast(path: &Path) -> Result<u64> {
    use rayon::prelude::*;

    // Parallel directory size calculation using Rayon's par_bridge()
    let total: u64 = walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .par_bridge() // Parallelize the iteration
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum();

    Ok(total)
}

/// Preview items that will be cleaned with a summary by category
fn preview_items(items: &[&CleanableItem]) {
    let total_size: u64 = items.iter().map(|item| item.size).sum();

    println!("\n{}", "=== Preview: Items to Clean ===".green().bold());
    println!(
        "Total space to free: {}\n",
        format_size(total_size, BINARY).bold()
    );

    // Group by category
    let mut by_category: HashMap<CleanCategory, Vec<&CleanableItem>> = HashMap::new();
    for item in items {
        by_category.entry(item.category).or_default().push(*item);
    }

    // Display by category
    let categories = [
        CleanCategory::SystemCache,
        CleanCategory::BrowserCache,
        CleanCategory::AppCache,
        CleanCategory::BrewCache,
        CleanCategory::PipCache,
        CleanCategory::CargoCache,
        CleanCategory::SystemLogs,
        CleanCategory::AppLogs,
        CleanCategory::NodeModules,
        CleanCategory::BuildArtifacts,
        CleanCategory::LargeFiles,
        CleanCategory::DuplicateFiles,
        CleanCategory::TempFiles,
    ];

    for category in categories {
        if let Some(cat_items) = by_category.get(&category) {
            let cat_total: u64 = cat_items.iter().map(|i| i.size).sum();
            let risk_indicator = match cat_items[0].risk_level {
                RiskLevel::Safe => "✓".green(),
                RiskLevel::Moderate => "⚠".yellow(),
                RiskLevel::Risky => "⚠".red(),
            };

            println!(
                "{} {} - {} ({} items)",
                risk_indicator,
                category,
                format_size(cat_total, BINARY).bold(),
                cat_items.len()
            );

            // Show up to 5 items per category
            for item in cat_items.iter().take(5) {
                println!(
                    "    {} - {}",
                    format_size(item.size, BINARY),
                    item.path.display().to_string().dimmed()
                );
            }
            if cat_items.len() > 5 {
                println!("    ... and {} more", cat_items.len() - 5);
            }
            println!();
        }
    }
}
