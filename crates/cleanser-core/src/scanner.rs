//! Filesystem scanning logic.
//!
//! This module provides the core scanning functionality that finds cleanable
//! files and directories. It uses callbacks for progress updates.

use crate::mapper::filesystem_map::{DirectoryCategory, MappedDirectory};
use crate::mapper::{FileSystemCrawler, FileSystemMap};
use crate::progress::{NoOpProgress, ProgressCallback, ScanPhase, ScanProgress};
use crate::types::*;
use crate::utils::get_dir_size;
use anyhow::Result;
use rayon::prelude::*;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tracing::warn;
use walkdir::WalkDir;

/// Scan using default settings (no progress callback)
pub fn scan(config: ScanConfig) -> Result<ScanResults> {
    scan_with_progress(config, &NoOpProgress)
}

/// Scan with progress callback
pub fn scan_with_progress(
    config: ScanConfig,
    progress: &dyn ProgressCallback,
) -> Result<ScanResults> {
    progress.on_scan_progress(ScanProgress {
        phase: ScanPhase::LoadingMap,
        message: "Starting filesystem scan...".into(),
        current: None,
        total: None,
    });

    // Load or create filesystem map
    let mut fs_map = FileSystemMap::load().unwrap_or_else(|_| {
        progress.on_scan_progress(ScanProgress {
            phase: ScanPhase::LoadingMap,
            message: "No filesystem map found. Creating initial map...".into(),
            current: None,
            total: None,
        });
        FileSystemMap::new()
    });

    // Check if map needs updating
    if fs_map.directories.is_empty() || fs_map.is_stale() {
        progress.on_scan_progress(ScanProgress {
            phase: ScanPhase::UpdatingMap,
            message: "Updating filesystem map...".into(),
            current: None,
            total: None,
        });

        let crawler = FileSystemCrawler::new()
            .with_max_depth(config.max_depth.unwrap_or(10))
            .with_min_confidence(0.6);

        if fs_map.directories.is_empty() {
            // First run: full crawl
            fs_map = crawler.crawl_full()?;
        } else {
            // Update stale entries
            crawler.smart_scan(&mut fs_map)?;
        }

        // Save the updated map
        if let Err(e) = fs_map.save() {
            warn!("Failed to save filesystem map: {}", e);
        }
    }

    progress.on_scan_progress(ScanProgress {
        phase: ScanPhase::Scanning,
        message: "Scanning with filesystem map...".into(),
        current: None,
        total: None,
    });

    // Use the map to guide scanning
    let mut items = scan_using_map(&config, &fs_map)?;

    // Separate pass for duplicates (requires hashing)
    if config.find_duplicates {
        progress.on_scan_progress(ScanProgress {
            phase: ScanPhase::FindingDuplicates,
            message: "Finding duplicate files...".into(),
            current: None,
            total: None,
        });
        let duplicate_items = find_duplicates(&config.paths, config.max_depth.unwrap_or(6))?;
        items.extend(duplicate_items);
    }

    progress.on_scan_progress(ScanProgress {
        phase: ScanPhase::Complete,
        message: "Scan complete!".into(),
        current: None,
        total: None,
    });

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
    if size < 1024 * 1024 {
        // < 1MB
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

    let description = format!(
        "{} ({})",
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
                if size > 1024 * 1024 {
                    // > 1MB
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

/// Find duplicate files using a two-phase approach:
/// 1. Group by size + partial hash (first and last 4KB) for quick pre-filtering
/// 2. Full SHA256 hash only for files that match in phase 1
fn find_duplicates(paths: &[PathBuf], max_depth: usize) -> Result<Vec<CleanableItem>> {
    // Phase 1: Group files by size
    let mut size_groups: HashMap<u64, Vec<PathBuf>> = HashMap::new();

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
                    // Only consider files larger than 1MB
                    if size > 1024 * 1024 {
                        size_groups
                            .entry(size)
                            .or_default()
                            .push(entry.path().to_path_buf());
                    }
                }
            }
        }
    }

    // Phase 2: For groups with multiple files, compute partial hash
    let mut partial_hash_groups: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for (size, file_paths) in size_groups {
        if file_paths.len() < 2 {
            continue; // No duplicates possible
        }

        for path in file_paths {
            if let Ok(partial) = compute_partial_hash(&path, size) {
                partial_hash_groups.entry(partial).or_default().push(path);
            }
        }
    }

    // Phase 3: For files with matching partial hash, compute full hash
    let mut full_hash_groups: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for (_, file_paths) in partial_hash_groups {
        if file_paths.len() < 2 {
            continue; // No duplicates possible
        }

        for path in file_paths {
            if let Ok(hash) = hash_file_full(&path) {
                full_hash_groups.entry(hash).or_default().push(path);
            }
        }
    }

    // Build duplicate items
    let mut duplicates = Vec::new();
    for (_, dup_paths) in full_hash_groups.iter() {
        if dup_paths.len() > 1 {
            // Keep the first file, mark others as duplicates
            for path in dup_paths.iter().skip(1) {
                if let Ok(metadata) = fs::metadata(path) {
                    duplicates.push(CleanableItem {
                        path: path.clone(),
                        size: metadata.len(),
                        category: CleanCategory::DuplicateFiles,
                        risk_level: RiskLevel::Risky,
                        description: format!(
                            "Duplicate of: {}",
                            dup_paths[0]
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

/// Compute a partial hash using first 4KB + last 4KB of the file
/// This is much faster than hashing the entire file and catches most differences
fn compute_partial_hash(path: &Path, file_size: u64) -> Result<String> {
    use std::io::{Seek, SeekFrom};

    const BLOCK_SIZE: usize = 4096;
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();

    // Include file size in hash to differentiate same-content-different-size edge cases
    hasher.update(file_size.to_le_bytes());

    // Read first block
    let mut buffer = [0u8; BLOCK_SIZE];
    let first_read = file.read(&mut buffer)?;
    hasher.update(&buffer[..first_read]);

    // Read last block (if file is large enough to have a different last block)
    if file_size > BLOCK_SIZE as u64 * 2 {
        file.seek(SeekFrom::End(-(BLOCK_SIZE as i64)))?;
        let last_read = file.read(&mut buffer)?;
        hasher.update(&buffer[..last_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Hash entire file using SHA256
fn hash_file_full(path: &Path) -> Result<String> {
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
        let is_nested = seen_prefixes
            .iter()
            .any(|prefix: &PathBuf| item.path.starts_with(prefix));

        if !is_nested {
            seen_prefixes.push(item.path.clone());
            result.push(item);
        }
    }

    result
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
