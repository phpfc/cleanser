use crate::types::*;
use crate::{cache, scanner};
use anyhow::Result;
use colored::Colorize;
use humansize::{format_size, BINARY};
use std::fs;

fn run_fresh_scan() -> Result<ScanResults> {
    let config = ScanConfig {
        speed: ScanSpeed::Normal,
        paths: vec![std::env::var("HOME")?],
        min_file_size_mb: 0, // Don't scan for large files during clean
        max_depth: Some(6),
        find_duplicates: false, // Don't look for duplicates during clean
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

    let total_size: u64 = items_to_clean.iter().map(|item| item.size).sum();

    println!("\n{}", "=== Items to Clean ===".green().bold());
    println!(
        "Total space to free: {}\n",
        format_size(total_size, BINARY).bold()
    );

    for item in &items_to_clean {
        let risk_indicator = match item.risk_level {
            RiskLevel::Safe => "✓".green(),
            RiskLevel::Moderate => "⚠".yellow(),
            RiskLevel::Risky => "⚠".red(),
        };

        println!(
            "{} {} - {} - {}",
            risk_indicator,
            item.category,
            format_size(item.size, BINARY),
            item.path.dimmed()
        );
    }

    println!();

    if dry_run {
        println!("{}", "DRY RUN: No files were deleted.".yellow().bold());
        return Ok(());
    }

    // Perform the cleanup
    let mut cleaned_size = 0u64;
    let mut cleaned_count = 0usize;
    let mut failed_count = 0usize;

    for item in items_to_clean {
        match delete_item(&item.path) {
            Ok(size) => {
                cleaned_size += size;
                cleaned_count += 1;
                println!("{} Cleaned: {}", "✓".green(), item.path.dimmed());
            }
            Err(e) => {
                failed_count += 1;
                println!("{} Failed to clean {}: {}", "✗".red(), item.path, e);
            }
        }
    }

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

    Ok(())
}

fn delete_item(path: &str) -> Result<u64> {
    let path = std::path::Path::new(path);

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

fn get_dir_size_fast(path: &std::path::Path) -> Result<u64> {
    let mut total = 0;

    for entry in walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                total += metadata.len();
            }
        }
    }

    Ok(total)
}
