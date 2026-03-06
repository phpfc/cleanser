use crate::types::*;
use anyhow::Result;
use colored::Colorize;
use humansize::{format_size, BINARY};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn scan(config: ScanConfig) -> Result<ScanResults> {
    println!("{}", "Starting dynamic filesystem scan...".cyan());

    // Determine max depth based on speed
    let max_depth = config.max_depth.unwrap_or(match config.speed {
        ScanSpeed::Quick => 3,
        ScanSpeed::Normal => 6,
        ScanSpeed::Thorough => usize::MAX,
    });

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("Failed to create progress bar template"),
    );

    // Single-pass scan for most items (cache, artifacts, logs, large files)
    pb.set_message("Scanning filesystem...");
    let mut items = scan_filesystem(&config, max_depth)?;

    // Separate pass for duplicates (requires hashing)
    if config.find_duplicates {
        pb.set_message("Finding duplicate files...");
        let duplicate_items = find_duplicates(&config.paths, max_depth)?;
        items.extend(duplicate_items);
    }

    pb.finish_with_message("Scan complete!".green().to_string());

    // Deduplicate nested paths to avoid double-counting
    let mut items = deduplicate_nested_paths(items);

    // Apply size range filter if specified
    let mut filtered_by_size_count = 0;
    if let Some(ref size_range) = config.size_range {
        let original_count = items.len();
        items.retain(|item| size_range.contains(item.size));
        filtered_by_size_count = original_count - items.len();
    }

    // Apply age filter if specified
    let mut filtered_by_age_count = 0;
    if let Some(ref age_criteria) = config.age_criteria {
        let original_count = items.len();
        items.retain(|item| {
            // Try to get modification time for the file
            if let Ok(metadata) = fs::metadata(&item.path) {
                if let Ok(modified) = metadata.modified() {
                    return age_criteria.matches(modified);
                }
            }
            // If we can't get modification time, skip the file
            false
        });
        filtered_by_age_count = original_count - items.len();
    }

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

/// Single-pass filesystem scan that checks for all item types
fn scan_filesystem(config: &ScanConfig, max_depth: usize) -> Result<Vec<CleanableItem>> {
    let cache_patterns = compile_cache_patterns();
    let artifact_patterns = get_artifact_patterns();
    let log_regex = Regex::new(r"\.log$")
        .expect("Invalid regex pattern for log files");
    let min_large_file_size = config.min_file_size_mb * 1024 * 1024;

    let skip_dirs = [
        "Library/Application Support",
        "Library/Mobile Documents",
        "Applications",
        "/System",
        "/Library",
        "Library/Mail",
    ];

    let items: Vec<CleanableItem> = config
        .paths
        .par_iter()
        .flat_map(|base_path| {
            WalkDir::new(base_path)
                .max_depth(max_depth)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| match e {
                    Ok(entry) => Some(entry),
                    Err(err) => {
                        // Only show non-permission errors
                        let err_str = err.to_string();
                        if !err_str.contains("Operation not permitted") &&
                           !err_str.contains("Permission denied") {
                            eprintln!("Warning: {}", err);
                        }
                        None
                    }
                })
                .filter_map(|entry| {
                    let path = entry.path();
                    let path_str = path.to_string_lossy();

                    // Check ignore patterns first
                    if config.ignore_patterns.should_ignore(path) {
                        return None;
                    }

                    // Skip our own target directory
                    if path_str.contains("/cleanser/target") {
                        return None;
                    }

                    // Skip forbidden directories for large file scan
                    if skip_dirs.iter().any(|skip| path_str.contains(skip)) {
                        if let Some(name) = path.file_name() {
                            let name_str = name.to_string_lossy();
                            if name_str.starts_with('.') && name_str != ".cache" {
                                return None;
                            }
                        }
                    }

                    if entry.file_type().is_dir() {
                        // Check cache directories
                        if let Some(item) = check_cache_directory(path, &cache_patterns) {
                            return Some(item);
                        }

                        // Check build artifacts
                        if let Some(item) = check_build_artifact(path, &artifact_patterns) {
                            return Some(item);
                        }
                    } else if entry.file_type().is_file() {
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

    Ok(items)
}

/// Helper function to compile cache regex patterns
fn compile_cache_patterns() -> Vec<Regex> {
    let cache_patterns = [
        r"(?i)cache$",
        r"(?i)\.cache$",
        r"(?i)caches$",
        r"Library/Caches",
    ];

    cache_patterns
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect()
}

/// Helper function to get artifact patterns
fn get_artifact_patterns() -> Vec<(&'static str, CleanCategory, RiskLevel)> {
    vec![
        ("node_modules", CleanCategory::NodeModules, RiskLevel::Moderate),
        ("target", CleanCategory::BuildArtifacts, RiskLevel::Moderate),
        ("build", CleanCategory::BuildArtifacts, RiskLevel::Moderate),
        ("dist", CleanCategory::BuildArtifacts, RiskLevel::Moderate),
        (".gradle", CleanCategory::BuildArtifacts, RiskLevel::Moderate),
        (".maven", CleanCategory::BuildArtifacts, RiskLevel::Moderate),
        ("__pycache__", CleanCategory::BuildArtifacts, RiskLevel::Safe),
        (".pytest_cache", CleanCategory::BuildArtifacts, RiskLevel::Safe),
        (".next", CleanCategory::BuildArtifacts, RiskLevel::Moderate),
        (".nuxt", CleanCategory::BuildArtifacts, RiskLevel::Moderate),
        ("out", CleanCategory::BuildArtifacts, RiskLevel::Moderate),
    ]
}

/// Check if a directory is a cache directory
fn check_cache_directory(path: &Path, regexes: &[Regex]) -> Option<CleanableItem> {
    let path_str = path.to_string_lossy();

    // Special handling for Library/Caches - scan inside for individual app caches
    // instead of trying to delete the protected parent directory
    if path_str.ends_with("Library/Caches") {
        // Don't return the parent Library/Caches directory itself
        // WalkDir will continue scanning inside and we'll catch individual app caches
        return None;
    }

    // Check if this is an individual app cache inside Library/Caches
    if let Some(parent) = path.parent() {
        let parent_str = parent.to_string_lossy();
        if parent_str.ends_with("Library/Caches") {
            // This is a direct child of Library/Caches (e.g., com.google.Chrome)
            if let Ok(size) = get_dir_size(path) {
                if size > 1024 * 1024 { // > 1MB
                    return Some(CleanableItem {
                        path: path.to_path_buf(),
                        size,
                        category: CleanCategory::SystemCache,
                        risk_level: RiskLevel::Safe,
                        description: format!(
                            "App cache: {}",
                            path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "unknown".to_string())
                        ),
                    });
                }
            }
            return None;
        }
    }

    // For all other cache directories, use the existing logic
    for regex in regexes {
        if regex.is_match(&path_str) {
            if let Ok(size) = get_dir_size(path) {
                if size > 1024 * 1024 { // > 1MB
                    let category = categorize_cache(path);
                    let risk = match category {
                        CleanCategory::SystemCache => RiskLevel::Safe,
                        CleanCategory::BrowserCache => RiskLevel::Safe,
                        _ => RiskLevel::Safe,
                    };

                    return Some(CleanableItem {
                        path: path.to_path_buf(),
                        size,
                        category,
                        risk_level: risk,
                        description: format!(
                            "Cache directory: {}",
                            path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "unknown".to_string())
                        ),
                    });
                }
            }
            break;
        }
    }
    None
}

/// Check if a directory is a build artifact
fn check_build_artifact(
    path: &Path,
    patterns: &[(&str, CleanCategory, RiskLevel)],
) -> Option<CleanableItem> {
    let dir_name = match path.file_name() {
        Some(name) => name.to_string_lossy(),
        None => return None,
    };

    for (pattern, category, risk) in patterns {
        if dir_name == *pattern {
            // Special handling for 'target' - check if it's a Rust project
            if *pattern == "target" {
                if let Some(parent) = path.parent() {
                    if !parent.join("Cargo.toml").exists() {
                        continue;
                    }
                }
            }

            // Special handling for 'build', 'dist', 'out' - check for project files
            if *pattern == "build" || *pattern == "dist" || *pattern == "out" {
                if let Some(parent) = path.parent() {
                    let has_project_file = parent.join("package.json").exists()
                        || parent.join("build.gradle").exists()
                        || parent.join("pom.xml").exists()
                        || parent.join("go.mod").exists();

                    if !has_project_file {
                        continue;
                    }
                }
            }

            if let Ok(size) = get_dir_size(path) {
                if size > 1024 * 1024 {
                    return Some(CleanableItem {
                        path: path.to_path_buf(),
                        size,
                        category: *category,
                        risk_level: *risk,
                        description: format!("{} directory", pattern),
                    });
                }
            }
            break;
        }
    }
    None
}

/// Check if a file is a large log file
fn check_log_file(path: &Path, log_regex: &Regex) -> Option<CleanableItem> {
    // Only check in log directories
    let path_str = path.to_string_lossy();
    if !path_str.contains("/Library/Logs") && !path_str.contains("/logs") && !path_str.contains("/.logs") {
        return None;
    }

    if log_regex.is_match(&path_str) {
        if let Ok(metadata) = fs::metadata(path) {
            let size = metadata.len();
            if size > 10 * 1024 * 1024 { // > 10MB
                return Some(CleanableItem {
                    path: path.to_path_buf(),
                    size,
                    category: if path_str.contains("Library/Logs") {
                        CleanCategory::SystemLogs
                    } else {
                        CleanCategory::AppLogs
                    },
                    risk_level: RiskLevel::Safe,
                    description: format!("Large log file ({})", format_size(size, BINARY)),
                });
            }
        }
    }
    None
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
                description: format!("Large file ({})", format_size(size, BINARY)),
            });
        }
    }
    None
}

fn deduplicate_nested_paths(items: Vec<CleanableItem>) -> Vec<CleanableItem> {
    let mut sorted_items = items;

    // Sort by path component count (shortest first) so parent directories come before their children
    sorted_items.sort_by(|a, b| {
        let a_components = a.path.components().count();
        let b_components = b.path.components().count();
        a_components.cmp(&b_components)
    });

    let mut deduplicated = Vec::new();

    for item in sorted_items {
        // Check if this item is a child of any already-kept item
        let is_child = deduplicated.iter().any(|kept: &CleanableItem| {
            // An item is a child if it starts with a kept path and is not the same path
            item.path.starts_with(&kept.path) && item.path != kept.path
        });

        // Only keep items that are not children of already-kept items
        if !is_child {
            deduplicated.push(item);
        }
    }

    deduplicated
}

fn find_duplicates(paths: &[PathBuf], max_depth: usize) -> Result<Vec<CleanableItem>> {
    let mut files_to_hash = Vec::new();

    for base_path in paths {
        for entry in WalkDir::new(base_path)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| match e {
                Ok(entry) => Some(entry),
                Err(err) => {
                    // Only show non-permission errors
                    let err_str = err.to_string();
                    if !err_str.contains("Operation not permitted") &&
                       !err_str.contains("Permission denied") {
                        eprintln!("Warning: {}", err);
                    }
                    None
                }
            })
        {
            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    let size = metadata.len();
                    if size > 1024 * 1024 {
                        files_to_hash.push((entry.path().to_path_buf(), size));
                    }
                }
            }
        }
    }

    // Hash files in parallel and collect into a HashMap
    let file_hashes: Vec<(FileHash, PathBuf)> = files_to_hash
        .par_iter()
        .filter_map(|(path, size)| {
            hash_file(path).ok().map(|hash| {
                (FileHash { hash, size: *size }, path.clone())
            })
        })
        .collect();

    // Group by hash
    let mut file_map: HashMap<FileHash, Vec<PathBuf>> = HashMap::new();
    for (file_hash, path) in file_hashes {
        file_map.entry(file_hash).or_default().push(path);
    }

    // Create CleanableItems for duplicates
    let mut duplicate_items = Vec::new();
    for (file_hash, paths_list) in file_map.iter() {
        if paths_list.len() > 1 {
            for path in paths_list.iter().skip(1) {
                duplicate_items.push(CleanableItem {
                    path: path.clone(),
                    size: file_hash.size,
                    category: CleanCategory::DuplicateFiles,
                    risk_level: RiskLevel::Risky,
                    description: format!(
                        "Duplicate of {} ({})",
                        paths_list[0].display(),
                        format_size(file_hash.size, BINARY)
                    ),
                });
            }
        }
    }

    Ok(duplicate_items)
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn get_dir_size(path: &Path) -> Result<u64> {
    // Parallel directory size calculation using Rayon's par_bridge()
    let total: u64 = WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .par_bridge()  // Parallelize the iteration
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum();

    Ok(total)
}

fn categorize_cache(path: &Path) -> CleanCategory {
    let path_str = path.to_string_lossy().to_lowercase();

    if path_str.contains("chrome") || path_str.contains("firefox") || path_str.contains("safari") {
        CleanCategory::BrowserCache
    } else if path_str.contains("homebrew") || path_str.contains("brew") {
        CleanCategory::BrewCache
    } else if path_str.contains("pip") {
        CleanCategory::PipCache
    } else if path_str.contains("cargo") {
        CleanCategory::CargoCache
    } else if path_str.contains("npm") || path_str.contains("yarn") || path_str.contains("pnpm") {
        CleanCategory::AppCache
    } else if path_str.contains("library/caches") {
        CleanCategory::SystemCache
    } else {
        CleanCategory::AppCache
    }
}

pub fn display_results(results: &ScanResults) {
    println!("\n{}", "=== Scan Results ===".green().bold());
    println!(
        "Total cleanable space: {}\n",
        format_size(results.total_size, BINARY).bold()
    );

    // Group by risk level
    let mut by_risk: HashMap<RiskLevel, Vec<&CleanableItem>> = HashMap::new();
    for item in &results.items {
        by_risk.entry(item.risk_level).or_default().push(item);
    }

    // Display by risk level
    for risk in [RiskLevel::Safe, RiskLevel::Moderate, RiskLevel::Risky] {
        if let Some(items) = by_risk.get(&risk) {
            let total: u64 = items.iter().map(|i| i.size).sum();

            let risk_color = match risk {
                RiskLevel::Safe => "green",
                RiskLevel::Moderate => "yellow",
                RiskLevel::Risky => "red",
            };

            println!(
                "{} ({}, {} items)",
                format!("{:?} Risk", risk).color(risk_color).bold(),
                format_size(total, BINARY).bold(),
                items.len()
            );

            // Group by category within risk level
            let mut by_category: HashMap<CleanCategory, Vec<&CleanableItem>> = HashMap::new();
            for item in items {
                by_category.entry(item.category).or_default().push(item);
            }

            for (category, cat_items) in by_category {
                let cat_total: u64 = cat_items.iter().map(|i| i.size).sum();
                println!(
                    "  {} - {} ({} items)",
                    category,
                    format_size(cat_total, BINARY),
                    cat_items.len()
                );

                // Show top 3 items in this category
                let mut sorted_items = cat_items.clone();
                sorted_items.sort_by(|a, b| b.size.cmp(&a.size));
                for item in sorted_items.iter().take(3) {
                    println!(
                        "    {} - {}",
                        format_size(item.size, BINARY),
                        item.path.display().to_string().dimmed()
                    );
                }
                if cat_items.len() > 3 {
                    println!("    ... and {} more", cat_items.len() - 3);
                }
            }
            println!();
        }
    }

    println!(
        "\n{}",
        "Run 'cleanser clean --risk <level>' to clean files".cyan()
    );
}
