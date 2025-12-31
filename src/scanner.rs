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
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

pub fn scan(config: ScanConfig) -> Result<ScanResults> {
    let items = Arc::new(Mutex::new(Vec::new()));

    println!("{}", "Starting dynamic filesystem scan...".cyan());

    // Determine max depth based on speed
    let max_depth = config.max_depth.unwrap_or_else(|| match config.speed {
        ScanSpeed::Quick => 3,
        ScanSpeed::Normal => 6,
        ScanSpeed::Thorough => usize::MAX,
    });

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );

    // 1. Scan for cache directories
    pb.set_message("Scanning for cache directories...");
    scan_cache_directories(&config.paths, max_depth, &items)?;

    // 2. Scan for build artifacts
    pb.set_message("Scanning for build artifacts...");
    scan_build_artifacts(&config.paths, max_depth, &items)?;

    // 3. Scan for log files
    pb.set_message("Scanning for log files...");
    scan_log_files(&config.paths, max_depth, &items)?;

    // 4. Scan for large files
    if config.min_file_size_mb > 0 {
        pb.set_message(format!(
            "Scanning for files larger than {}MB...",
            config.min_file_size_mb
        ));
        scan_large_files(&config.paths, max_depth, config.min_file_size_mb, &items)?;
    }

    // 5. Find duplicates
    if config.find_duplicates {
        pb.set_message("Finding duplicate files...");
        find_duplicates(&config.paths, max_depth, &items)?;
    }

    pb.finish_with_message("Scan complete!".green().to_string());

    let items = Arc::try_unwrap(items).unwrap().into_inner().unwrap();

    // Deduplicate nested paths to avoid double-counting
    let items = deduplicate_nested_paths(items);

    let total_size: u64 = items.iter().map(|item| item.size).sum();

    Ok(ScanResults {
        items,
        total_size,
        scan_speed: config.speed,
    })
}

fn deduplicate_nested_paths(items: Vec<CleanableItem>) -> Vec<CleanableItem> {
    let mut sorted_items = items;

    // Sort by path length (shortest first) so parent directories come before their children
    sorted_items.sort_by(|a, b| a.path.len().cmp(&b.path.len()));

    let mut deduplicated = Vec::new();

    for item in sorted_items {
        let path = Path::new(&item.path);

        // Check if this item is a child of any already-kept item
        let is_child = deduplicated.iter().any(|kept: &CleanableItem| {
            let kept_path = Path::new(&kept.path);
            // An item is a child if it starts with a kept path and is not the same path
            path.starts_with(kept_path) && path != kept_path
        });

        // Only keep items that are not children of already-kept items
        if !is_child {
            deduplicated.push(item);
        }
    }

    deduplicated
}

fn scan_cache_directories(
    paths: &[String],
    max_depth: usize,
    items: &Arc<Mutex<Vec<CleanableItem>>>,
) -> Result<()> {
    let cache_patterns = vec![
        r"(?i)cache$",
        r"(?i)\.cache$",
        r"(?i)caches$",
        r"Library/Caches",
    ];

    let regexes: Vec<Regex> = cache_patterns
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect();

    for base_path in paths {
        for entry in WalkDir::new(base_path)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_dir() {
                continue;
            }

            let path = entry.path();
            let path_str = path.to_string_lossy();

            // Skip our own target directory
            if path_str.contains("/target/") || path_str.contains("/cleanser/") {
                continue;
            }

            for regex in &regexes {
                if regex.is_match(&path_str) {
                    if let Ok(size) = get_dir_size(path) {
                        if size > 1024 * 1024 {
                            // > 1MB
                            let category = categorize_cache(path);
                            let risk = match category {
                                CleanCategory::SystemCache => RiskLevel::Safe,
                                CleanCategory::BrowserCache => RiskLevel::Safe,
                                _ => RiskLevel::Safe,
                            };

                            items.lock().unwrap().push(CleanableItem {
                                path: path.display().to_string(),
                                size,
                                category,
                                risk_level: risk,
                                description: format!("Cache directory: {}", path.file_name().unwrap_or_default().to_string_lossy()),
                            });
                        }
                    }
                    break;
                }
            }
        }
    }

    Ok(())
}

fn scan_build_artifacts(
    paths: &[String],
    max_depth: usize,
    items: &Arc<Mutex<Vec<CleanableItem>>>,
) -> Result<()> {
    let artifact_patterns = vec![
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
    ];

    for base_path in paths {
        for entry in WalkDir::new(base_path)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_dir() {
                continue;
            }

            let path = entry.path();
            let path_str = path.to_string_lossy();

            // Skip our own target directory
            if path_str.contains("/cleanser/target") {
                continue;
            }

            let dir_name = path.file_name().unwrap_or_default().to_string_lossy();

            for (pattern, category, risk) in &artifact_patterns {
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
                            items.lock().unwrap().push(CleanableItem {
                                path: path.display().to_string(),
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
        }
    }

    Ok(())
}

fn scan_log_files(
    paths: &[String],
    _max_depth: usize,
    items: &Arc<Mutex<Vec<CleanableItem>>>,
) -> Result<()> {
    let log_regex = Regex::new(r"\.log$").unwrap();

    for base_path in paths {
        let log_paths = vec![
            format!("{}/Library/Logs", base_path),
            format!("{}/logs", base_path),
            format!("{}/.logs", base_path),
        ];

        for log_path in log_paths {
            if !Path::new(&log_path).exists() {
                continue;
            }

            for entry in WalkDir::new(&log_path)
                .max_depth(3)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();

                if entry.file_type().is_file() && log_regex.is_match(&path.to_string_lossy()) {
                    if let Ok(metadata) = fs::metadata(path) {
                        let size = metadata.len();
                        if size > 10 * 1024 * 1024 {
                            items.lock().unwrap().push(CleanableItem {
                                path: path.display().to_string(),
                                size,
                                category: if path.to_string_lossy().contains("Library/Logs") {
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
            }
        }
    }

    Ok(())
}

fn scan_large_files(
    paths: &[String],
    max_depth: usize,
    min_size_mb: u64,
    items: &Arc<Mutex<Vec<CleanableItem>>>,
) -> Result<()> {
    let min_size = min_size_mb * 1024 * 1024;

    let skip_dirs = vec![
        "Library/Application Support",
        "Library/Mobile Documents",
        "Applications",
        "/System",
        "/Library",
        "Library/Mail",
    ];

    for base_path in paths {
        for entry in WalkDir::new(base_path)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let path_str = path.to_string_lossy();

            if skip_dirs.iter().any(|skip| path_str.contains(skip)) {
                continue;
            }

            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str.starts_with('.') && name_str != ".cache" {
                    continue;
                }
            }

            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    let size = metadata.len();
                    if size >= min_size {
                        items.lock().unwrap().push(CleanableItem {
                            path: path.display().to_string(),
                            size,
                            category: CleanCategory::LargeFiles,
                            risk_level: RiskLevel::Risky,
                            description: format!("Large file ({})", format_size(size, BINARY)),
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

fn find_duplicates(
    paths: &[String],
    max_depth: usize,
    items: &Arc<Mutex<Vec<CleanableItem>>>,
) -> Result<()> {
    let file_map: Arc<Mutex<HashMap<FileHash, Vec<PathBuf>>>> = Arc::new(Mutex::new(HashMap::new()));

    let mut files_to_hash = Vec::new();

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
                    if size > 1024 * 1024 {
                        files_to_hash.push((entry.path().to_path_buf(), size));
                    }
                }
            }
        }
    }

    files_to_hash
        .par_iter()
        .for_each(|(path, size)| {
            if let Ok(hash) = hash_file(path) {
                let file_hash = FileHash {
                    hash,
                    size: *size,
                };
                file_map
                    .lock()
                    .unwrap()
                    .entry(file_hash)
                    .or_insert_with(Vec::new)
                    .push(path.clone());
            }
        });

    let file_map = file_map.lock().unwrap();
    for (file_hash, paths_list) in file_map.iter() {
        if paths_list.len() > 1 {
            for path in paths_list.iter().skip(1) {
                items.lock().unwrap().push(CleanableItem {
                    path: path.display().to_string(),
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

    Ok(())
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
    let mut total = 0;

    for entry in WalkDir::new(path)
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
        by_risk
            .entry(item.risk_level)
            .or_insert_with(Vec::new)
            .push(item);
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
                by_category
                    .entry(item.category)
                    .or_insert_with(Vec::new)
                    .push(item);
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
                        item.path.dimmed()
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
        format!("Run 'cleanser clean --risk <level>' to clean files").cyan()
    );
}
