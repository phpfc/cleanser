//! File and directory cleaning logic.
//!
//! This module provides the core cleaning functionality that deletes
//! files and directories. It uses callbacks for progress updates.

use crate::progress::{CleanPhase, CleanProgress, NoOpProgress, ProgressCallback};
use crate::types::*;
use crate::utils::get_dir_size;
use crate::{cache, platform, scanner};
use anyhow::Result;
use std::fs;
use std::path::Path;
use tracing::warn;

/// Clean files without progress callback
pub fn clean(max_risk: RiskLevel, dry_run: bool, force_scan: bool) -> Result<CleanResult> {
    clean_with_progress(max_risk, dry_run, force_scan, &NoOpProgress)
}

/// Clean files with progress callback
pub fn clean_with_progress(
    max_risk: RiskLevel,
    dry_run: bool,
    force_scan: bool,
    progress: &dyn ProgressCallback,
) -> Result<CleanResult> {
    // Try to load from cache first
    let results = if !force_scan {
        progress.on_clean_progress(CleanProgress {
            phase: CleanPhase::Loading,
            message: "Loading cached scan results...".into(),
            current_item: None,
            current: 0,
            total: 0,
            cleaned_size: 0,
        });

        match cache::load_scan_results(None) {
            Ok(Some(cached_results)) => cached_results,
            Ok(None) => {
                progress.on_clean_progress(CleanProgress {
                    phase: CleanPhase::Scanning,
                    message: "No cached scan found, running fresh scan...".into(),
                    current_item: None,
                    current: 0,
                    total: 0,
                    cleaned_size: 0,
                });
                run_fresh_scan()?
            }
            Err(_) => {
                progress.on_clean_progress(CleanProgress {
                    phase: CleanPhase::Scanning,
                    message: "Failed to load cache, running fresh scan...".into(),
                    current_item: None,
                    current: 0,
                    total: 0,
                    cleaned_size: 0,
                });
                run_fresh_scan()?
            }
        }
    } else {
        progress.on_clean_progress(CleanProgress {
            phase: CleanPhase::Scanning,
            message: "Running fresh scan (--force-scan)...".into(),
            current_item: None,
            current: 0,
            total: 0,
            cleaned_size: 0,
        });
        run_fresh_scan()?
    };

    // Filter items by risk level
    let items_to_clean: Vec<&CleanableItem> = results
        .items
        .iter()
        .filter(|item| item.risk_level <= max_risk)
        .collect();

    if items_to_clean.is_empty() {
        return Ok(CleanResult::default());
    }

    if dry_run {
        // In dry run, just return what would be cleaned
        let total_size: u64 = items_to_clean.iter().map(|i| i.size).sum();
        return Ok(CleanResult {
            cleaned_count: items_to_clean.len(),
            failed_count: 0,
            cleaned_size: total_size,
            deleted_paths: items_to_clean.iter().map(|i| i.path.clone()).collect(),
            failures: Vec::new(),
        });
    }

    // Perform the cleanup
    let mut result = CleanResult::default();
    let total = items_to_clean.len() as u64;

    for (idx, item) in items_to_clean.iter().enumerate() {
        progress.on_clean_progress(CleanProgress {
            phase: CleanPhase::Cleaning,
            message: format!("Cleaning: {}", item.path.display()),
            current_item: Some(item.path.to_string_lossy().to_string()),
            current: idx as u64 + 1,
            total,
            cleaned_size: result.cleaned_size,
        });

        match delete_item(&item.path) {
            Ok(size) => {
                result.cleaned_size += size;
                result.cleaned_count += 1;
                result.deleted_paths.push(item.path.clone());
            }
            Err(e) => {
                result.failed_count += 1;
                result.failures.push((item.path.clone(), e.to_string()));
            }
        }
    }

    progress.on_clean_progress(CleanProgress {
        phase: CleanPhase::Complete,
        message: format!(
            "Cleanup complete! {} items cleaned, {} failed",
            result.cleaned_count, result.failed_count
        ),
        current_item: None,
        current: total,
        total,
        cleaned_size: result.cleaned_size,
    });

    // Update cache to remove deleted items
    if !result.deleted_paths.is_empty() && !force_scan {
        if let Err(e) = cache::update_cache_after_deletion(&result.deleted_paths) {
            warn!("Failed to update cache after deletion: {}", e);
        }
    }

    Ok(result)
}

/// Run a fresh scan with default settings
fn run_fresh_scan() -> Result<ScanResults> {
    let mut ignore_patterns = IgnoreList::new();

    // Load whitelist and add to ignore patterns
    if let Ok(whitelist) = WhitelistConfig::load() {
        for path in whitelist.list_paths() {
            if let Err(e) = ignore_patterns.add_pattern(&path.to_string_lossy()) {
                warn!("Failed to add whitelist pattern {}: {}", path.display(), e);
            }
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
    };

    let results = scanner::scan(config)?;

    // Save to cache for next time
    if let Err(e) = cache::save_scan_results(&results) {
        warn!("Failed to save scan results to cache: {}", e);
    }

    Ok(results)
}

/// Check if a path or any of its contents contain symlinks pointing outside
fn contains_external_symlinks(path: &Path) -> Result<bool> {
    let canonical_base = path.canonicalize()?;

    if path.is_dir() {
        for entry in walkdir::WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_path = entry.path();
            if entry_path.is_symlink() {
                // Check if symlink points outside the directory being deleted
                if let Ok(target) = fs::read_link(entry_path) {
                    let absolute_target = if target.is_absolute() {
                        target
                    } else {
                        entry_path.parent().unwrap_or(path).join(&target)
                    };

                    if let Ok(canonical_target) = absolute_target.canonicalize() {
                        if !canonical_target.starts_with(&canonical_base) {
                            warn!(
                                "Found symlink pointing outside directory: {} -> {}",
                                entry_path.display(),
                                canonical_target.display()
                            );
                            return Ok(true);
                        }
                    }
                }
            }
        }
    } else if path.is_symlink() {
        // Single file symlink - just warn but allow deletion
        warn!("Path is a symlink: {}", path.display());
    }

    Ok(false)
}

/// Delete a file or directory
fn delete_item(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }

    // Safety check: verify no symlinks point outside the directory
    if path.is_dir() {
        if contains_external_symlinks(path)? {
            anyhow::bail!(
                "Directory contains symlinks pointing outside. Refusing to delete for safety: {}",
                path.display()
            );
        }
    }

    // Calculate size before deletion
    let size = if path.is_dir() {
        get_dir_size(path)?
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

/// Delete specific items (used by interactive mode)
pub fn delete_items(items: &[CleanableItem], dry_run: bool) -> Result<CleanResult> {
    delete_items_with_progress(items, dry_run, &NoOpProgress)
}

/// Delete specific items with progress callback
pub fn delete_items_with_progress(
    items: &[CleanableItem],
    dry_run: bool,
    progress: &dyn ProgressCallback,
) -> Result<CleanResult> {
    if dry_run {
        let total_size: u64 = items.iter().map(|i| i.size).sum();
        return Ok(CleanResult {
            cleaned_count: items.len(),
            failed_count: 0,
            cleaned_size: total_size,
            deleted_paths: items.iter().map(|i| i.path.clone()).collect(),
            failures: Vec::new(),
        });
    }

    let mut result = CleanResult::default();
    let total = items.len() as u64;

    for (idx, item) in items.iter().enumerate() {
        progress.on_clean_progress(CleanProgress {
            phase: CleanPhase::Cleaning,
            message: format!("Deleting: {}", item.path.display()),
            current_item: Some(item.path.to_string_lossy().to_string()),
            current: idx as u64 + 1,
            total,
            cleaned_size: result.cleaned_size,
        });

        let del_result = if item.path.is_dir() {
            fs::remove_dir_all(&item.path)
        } else {
            fs::remove_file(&item.path)
        };

        match del_result {
            Ok(_) => {
                result.cleaned_size += item.size;
                result.cleaned_count += 1;
                result.deleted_paths.push(item.path.clone());
            }
            Err(e) => {
                result.failed_count += 1;
                result.failures.push((item.path.clone(), e.to_string()));
            }
        }
    }

    progress.on_clean_progress(CleanProgress {
        phase: CleanPhase::Complete,
        message: format!(
            "Deletion complete! {} items deleted, {} failed",
            result.cleaned_count, result.failed_count
        ),
        current_item: None,
        current: total,
        total,
        cleaned_size: result.cleaned_size,
    });

    Ok(result)
}

/// Get items filtered by risk level from scan results
pub fn filter_by_risk(results: &ScanResults, max_risk: RiskLevel) -> Vec<CleanableItem> {
    results
        .items
        .iter()
        .filter(|item| item.risk_level <= max_risk)
        .cloned()
        .collect()
}
