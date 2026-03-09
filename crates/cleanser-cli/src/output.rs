//! Output formatting for CLI.

use cleanser_core::{CleanCategory, CleanableItem, RiskLevel, ScanResults};
use colored::Colorize;
use humansize::{format_size, BINARY};
use std::collections::HashMap;

/// Display scan results in a formatted way
pub fn display_scan_results(results: &ScanResults) {
    println!();
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "                      SCAN RESULTS                         "
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════".cyan()
    );
    println!();

    if results.items.is_empty() {
        println!("  {}", "No cleanable items found.".yellow());
        println!();
        return;
    }

    // Group by category
    let mut by_category: HashMap<CleanCategory, Vec<&CleanableItem>> = HashMap::new();
    for item in &results.items {
        by_category.entry(item.category).or_default().push(item);
    }

    // Sort categories by total size
    let mut categories: Vec<_> = by_category.iter().collect();
    categories.sort_by(|a, b| {
        let size_a: u64 = a.1.iter().map(|i| i.size).sum();
        let size_b: u64 = b.1.iter().map(|i| i.size).sum();
        size_b.cmp(&size_a)
    });

    for (category, items) in categories {
        let total_size: u64 = items.iter().map(|i| i.size).sum();
        let risk_color = match items.first().map(|i| &i.risk_level) {
            Some(RiskLevel::Safe) => "green",
            Some(RiskLevel::Moderate) => "yellow",
            Some(RiskLevel::Risky) => "red",
            None => "white",
        };

        println!(
            "  {} {} ({})",
            format!("{}", category).color(risk_color).bold(),
            format!("({} items)", items.len()).dimmed(),
            format_size(total_size, BINARY).white().bold()
        );

        // Show top 5 largest items per category
        let mut sorted_items: Vec<_> = items.iter().collect();
        sorted_items.sort_by(|a, b| b.size.cmp(&a.size));

        for item in sorted_items.iter().take(5) {
            let path_str = item.path.to_string_lossy();
            let display_path = if path_str.len() > 50 {
                format!("...{}", &path_str[path_str.len() - 47..])
            } else {
                path_str.to_string()
            };

            println!(
                "    {:>10}  {}",
                format_size(item.size, BINARY).dimmed(),
                display_path
            );
        }

        if items.len() > 5 {
            println!("    {} {} more...", "".dimmed(), items.len() - 5);
        }
        println!();
    }

    // Summary
    println!(
        "{}",
        "───────────────────────────────────────────────────────────".cyan()
    );
    println!(
        "  {} {}",
        "Total cleanable:".white().bold(),
        format_size(results.total_size, BINARY).green().bold()
    );
    println!("  {} {}", "Items found:".dimmed(), results.items.len());

    if results.filtered_by_size_count > 0 {
        println!(
            "  {} {}",
            "Filtered by size:".dimmed(),
            results.filtered_by_size_count
        );
    }
    if results.filtered_by_age_count > 0 {
        println!(
            "  {} {}",
            "Filtered by age:".dimmed(),
            results.filtered_by_age_count
        );
    }
    println!();
}

/// Preview items that will be cleaned with a summary by category
#[allow(dead_code)]
pub fn preview_clean_items(items: &[&CleanableItem]) {
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

/// Format risk level with color
#[allow(dead_code)]
pub fn format_risk_level(risk: &RiskLevel) -> colored::ColoredString {
    match risk {
        RiskLevel::Safe => "safe".green(),
        RiskLevel::Moderate => "moderate".yellow(),
        RiskLevel::Risky => "risky".red(),
    }
}
