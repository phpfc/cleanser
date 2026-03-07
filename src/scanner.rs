use crate::mapper::{FileSystemCrawler, FileSystemMap};
use crate::mapper::filesystem_map::{DirectoryCategory, MappedDirectory};
use crate::types::*;
use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Scan using the filesystem mapper
pub fn scan(config: ScanConfig) -> Result<ScanResults> {
    println!("{}", "Starting filesystem scan...".cyan());

    // Load or create filesystem map
    let mut fs_map = FileSystemMap::load().unwrap_or_else(|_| {
        println!("{}", "No filesystem map found. Creating initial map...".yellow());
        FileSystemMap::new()
    });

    // Check if map needs updating
    if fs_map.directories.is_empty() || fs_map.is_stale() {
        println!("{}", "Updating filesystem map...".cyan());
        let crawler = FileSystemCrawler::new()
            .with_max_depth(config.max_depth.unwrap_or(10))
            .with_min_confidence(0.6)
            .with_progress(true);

        if fs_map.directories.is_empty() {
            // First run: full crawl
            fs_map = crawler.crawl_full()?;
        } else {
            // Update stale entries
            crawler.smart_scan(&mut fs_map)?;
        }

        // Save the updated map
        if let Err(e) = fs_map.save() {
            eprintln!("{}", format!("Warning: Could not save filesystem map: {}", e).yellow());
        }
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("Failed to create progress bar template"),
    );

    // Use the map to guide scanning
    pb.set_message("Scanning with filesystem map...");
    let mut items = scan_using_map(&config, &fs_map)?;

    // Separate pass for duplicates (requires hashing)
    if config.find_duplicates {
        pb.set_message("Finding duplicate files...");
        let duplicate_items = find_duplicates(&config.paths, config.max_depth.unwrap_or(6))?;
        items.extend(duplicate_items);
    }

    pb.finish_with_message("Scan complete!".green().to_string());

    // Deduplicate nested paths
    let mut items = deduplicate_nested_paths(items);

    // Apply filters
    let filtered_by_size_count = apply_size_filter(&mut items, &config);
    let filtered_by_age_count = apply_age_filter(&mut items, &config);

    let total_size: u64 = items.iter().map(|item| item.size).sum();

    Ok(ScanResults {
        items,
        total_size,
        scan_speed: config.speed,
        excluded_dirs_count: 0,
        filtered_by_size_count,
        filtered_by_age_count,
    })
}

/// Scan using the filesystem map for guidance
fn scan_using_map(config: &ScanConfig, fs_map: &FileSystemMap) -> Result<Vec<CleanableItem>> {
    let min_large_file_size = config.min_file_size_mb * 1024 * 1024;
    let log_regex = Regex::new(r"\.log$").expect("Invalid regex pattern for log files");

    let mut items = Vec::new();

    // First, scan directories identified in the map
    let dirs_vec: Vec<&MappedDirectory> = fs_map.directories.values().collect();
    let mapped_items: Vec<CleanableItem> = dirs_vec
        .par_iter()
        .filter_map(|&mapped_dir| {
            // Skip if in ignore list
            if config.ignore_patterns.should_ignore(&mapped_dir.path) {
                return None;
            }

            // Only process if confidence is high enough
            if mapped_dir.confidence < 0.7 {
                return None;
            }

            // Convert mapped directory to cleanable item
            map_to_cleanable_item(mapped_dir)
        })
        .collect();

    items.extend(mapped_items);

    // Then, scan configured paths for items not in the map (large files, logs)
    let max_depth = config.max_depth.unwrap_or(match config.speed {
        ScanSpeed::Quick => 3,
        ScanSpeed::Normal => 6,
        ScanSpeed::Thorough => usize::MAX,
    });

    let additional_items: Vec<CleanableItem> = config
        .paths
        .par_iter()
        .flat_map(|base_path| {
            WalkDir::new(base_path)
                .max_depth(max_depth)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter_map(|entry| {
                    let path = entry.path();

                    // Check ignore patterns
                    if config.ignore_patterns.should_ignore(path) {
                        return None;
                    }

                    // Only check for large files and logs here
                    // (directories are already in the map)
                    if entry.file_type().is_file() {
                        // Check log files
                        if let Some(item) = check_log_file(path, &log_regex) {
                            return Some(item);
                        }

                        // Check large files
                        if min_large_file_size > 0 {
                            if let Some(item) = check_large_file(path, min_large_file_size) {
                                return Some(item);
                            }
                        }
                    }

                    None
                })
                .collect::<Vec<_>>()
        })
        .collect();

    items.extend(additional_items);

    Ok(items)
}

/// Convert a MappedDirectory to a CleanableItem
fn map_to_cleanable_item(mapped: &MappedDirectory) -> Option<CleanableItem> {
    // Re-calculate size if needed (map might be stale)
    let size = if mapped.path.exists() {
        get_dir_size(&mapped.path).unwrap_or(mapped.estimated_size)
    } else {
        return None; // Path no longer exists
    };

    // Only include if significant size
    if size < 1024 * 1024 { // < 1MB
        return None;
    }

    // Map category to CleanCategory
    let (category, risk_level) = match mapped.category {
        DirectoryCategory::Ephemeral => (CleanCategory::SystemCache, RiskLevel::Safe),
        DirectoryCategory::BuildArtifact => {
            if mapped.has_tag("node_modules") {
                (CleanCategory::NodeModules, RiskLevel::Moderate)
            } else {
                (CleanCategory::BuildArtifacts, RiskLevel::Moderate)
            }
        }
        DirectoryCategory::ApplicationData => (CleanCategory::AppCache, RiskLevel::Safe),
        DirectoryCategory::System => return None, // Don't suggest cleaning system dirs
        DirectoryCategory::UserContent => return None, // Don't suggest cleaning user content
        DirectoryCategory::Unknown => return None, // Skip unknown
    };

    let description = format!("{} ({})",
        mapped.description(),
        mapped.path.file_name()?.to_string_lossy()
    );

    Some(CleanableItem {
        path: mapped.path.clone(),
        size,
        category,
        risk_level,
        description,
    })
}

/// Apply size filter to items
fn apply_size_filter(items: &mut Vec<CleanableItem>, config: &ScanConfig) -> usize {
    if let Some(ref size_range) = config.size_range {
        let original_count = items.len();
        items.retain(|item| size_range.contains(item.size));
        original_count - items.len()
    } else {
        0
    }
}

/// Apply age filter to items
fn apply_age_filter(items: &mut Vec<CleanableItem>, config: &ScanConfig) -> usize {
    if let Some(ref age_criteria) = config.age_criteria {
        let original_count = items.len();
        items.retain(|item| {
            if let Ok(metadata) = fs::metadata(&item.path) {
                if let Ok(modified) = metadata.modified() {
                    return age_criteria.matches(modified);
                }
            }
            false
        });
        original_count - items.len()
    } else {
        0
    }
}

/// Check if a file is a large file
fn check_large_file(path: &Path, min_size: u64) -> Option<CleanableItem> {
    if let Ok(metadata) = fs::metadata(path) {
        let size = metadata.len();
        if size >= min_size {
            return Some(CleanableItem {
                path: path.to_path_buf(),
                size,
                category: CleanCategory::LargeFiles,
                risk_level: RiskLevel::Risky,
                description: format!(
                    "Large file: {}",
                    path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ),
            });
        }
    }
    None
}

/// Check if a file is a log file
fn check_log_file(path: &Path, log_regex: &Regex) -> Option<CleanableItem> {
    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy();
        if log_regex.is_match(&name_str) {
            if let Ok(metadata) = fs::metadata(path) {
                let size = metadata.len();
                if size > 1024 * 1024 { // > 1MB
                    return Some(CleanableItem {
                        path: path.to_path_buf(),
                        size,
                        category: CleanCategory::SystemLogs,
                        risk_level: RiskLevel::Safe,
                        description: format!("Log file: {}", name_str),
                    });
                }
            }
        }
    }
    None
}

/// Get the total size of a directory
fn get_dir_size(path: &Path) -> Result<u64> {
    let mut total_size = 0u64;

    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
            }
        }
    }

    Ok(total_size)
}

/// Find duplicate files using SHA256 hashing
fn find_duplicates(paths: &[PathBuf], max_depth: usize) -> Result<Vec<CleanableItem>> {
    let mut file_hashes: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for base_path in paths {
        for entry in WalkDir::new(base_path)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    let size = metadata.len();
                    // Only hash files larger than 1MB to save time
                    if size > 1024 * 1024 {
                        if let Ok(hash) = hash_file(entry.path()) {
                            file_hashes
                                .entry(hash)
                                .or_insert_with(Vec::new)
                                .push(entry.path().to_path_buf());
                        }
                    }
                }
            }
        }
    }

    let mut duplicates = Vec::new();
    for (_, paths) in file_hashes.iter() {
        if paths.len() > 1 {
            // Keep the first file, mark others as duplicates
            for path in paths.iter().skip(1) {
                if let Ok(metadata) = fs::metadata(path) {
                    duplicates.push(CleanableItem {
                        path: path.clone(),
                        size: metadata.len(),
                        category: CleanCategory::DuplicateFiles,
                        risk_level: RiskLevel::Risky,
                        description: format!(
                            "Duplicate of: {}",
                            paths[0]
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "unknown".to_string())
                        ),
                    });
                }
            }
        }
    }

    Ok(duplicates)
}

/// Hash a file using SHA256
fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Deduplicate nested paths to avoid counting parent and child
fn deduplicate_nested_paths(mut items: Vec<CleanableItem>) -> Vec<CleanableItem> {
    // Sort by path length ascending (shorter paths first = parents first)
    // Then by size descending for equal lengths
    items.sort_by(|a, b| {
        let len_cmp = a.path.as_os_str().len().cmp(&b.path.as_os_str().len());
        if len_cmp == std::cmp::Ordering::Equal {
            b.size.cmp(&a.size)
        } else {
            len_cmp
        }
    });

    let mut result = Vec::new();
    let mut seen_prefixes = Vec::new();

    for item in items {
        let is_nested = seen_prefixes.iter().any(|prefix: &PathBuf| {
            item.path.starts_with(prefix)
        });

        if !is_nested {
            seen_prefixes.push(item.path.clone());
            result.push(item);
        }
    }

    result
}

/// Display scan results to the user
pub fn display_results(results: &ScanResults) {
    use humansize::{format_size, BINARY};

    println!();
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("{}", "                      SCAN RESULTS                         ".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!();

    if results.items.is_empty() {
        println!("  {}", "No cleanable items found.".yellow());
        println!();
        return;
    }

    // Group by category
    let mut by_category: HashMap<CleanCategory, Vec<&CleanableItem>> = HashMap::new();
    for item in &results.items {
        by_category
            .entry(item.category.clone())
            .or_default()
            .push(item);
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

        println!("  {} {} ({})",
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

            println!("    {:>10}  {}",
                format_size(item.size, BINARY).dimmed(),
                display_path
            );
        }

        if items.len() > 5 {
            println!("    {} {} more...",
                "".dimmed(),
                items.len() - 5
            );
        }
        println!();
    }

    // Summary
    println!("{}", "───────────────────────────────────────────────────────────".cyan());
    println!("  {} {}",
        "Total cleanable:".white().bold(),
        format_size(results.total_size, BINARY).green().bold()
    );
    println!("  {} {}",
        "Items found:".dimmed(),
        results.items.len()
    );

    if results.filtered_by_size_count > 0 {
        println!("  {} {}",
            "Filtered by size:".dimmed(),
            results.filtered_by_size_count
        );
    }
    if results.filtered_by_age_count > 0 {
        println!("  {} {}",
            "Filtered by age:".dimmed(),
            results.filtered_by_age_count
        );
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplicate_nested_paths() {
        let items = vec![
            CleanableItem {
                path: PathBuf::from("/tmp/cache"),
                size: 1000,
                category: CleanCategory::SystemCache,
                risk_level: RiskLevel::Safe,
                description: "test".to_string(),
            },
            CleanableItem {
                path: PathBuf::from("/tmp/cache/sub"),
                size: 500,
                category: CleanCategory::SystemCache,
                risk_level: RiskLevel::Safe,
                description: "test".to_string(),
            },
        ];

        let result = deduplicate_nested_paths(items);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("/tmp/cache"));
    }
}
