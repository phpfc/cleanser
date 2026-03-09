#![allow(dead_code)]
#![allow(unused_mut)]

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

use super::filesystem_map::{DirectoryCategory, FileSystemMap};
use super::heuristics::PathClassifier;
use crate::platform::{paths, Platform};
use crate::types::WhitelistConfig;

pub struct FileSystemCrawler {
    platform: Platform,
    max_depth: usize,
    min_confidence: f32,
    progress: Option<ProgressBar>,
    ignored_paths: Vec<PathBuf>,
}

impl FileSystemCrawler {
    pub fn new() -> Self {
        // Load whitelist automatically
        let ignored_paths = WhitelistConfig::load()
            .map(|w| w.list_paths().into_iter().cloned().collect())
            .unwrap_or_default();

        Self {
            platform: Platform::current(),
            max_depth: 10,
            min_confidence: 0.6,
            progress: None,
            ignored_paths,
        }
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn with_min_confidence(mut self, confidence: f32) -> Self {
        self.min_confidence = confidence;
        self
    }

    pub fn with_progress(mut self, show_progress: bool) -> Self {
        if show_progress {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} [{elapsed_precise}] {msg}")
                    .unwrap(),
            );
            self.progress = Some(pb);
        }
        self
    }

    /// Check if a path should be ignored (is in whitelist)
    fn is_ignored(&self, path: &Path) -> bool {
        for ignored in &self.ignored_paths {
            if path.starts_with(ignored) {
                return true;
            }
        }
        false
    }

    /// Perform initial full filesystem crawl
    pub fn crawl_full(&self) -> Result<FileSystemMap> {
        let mut map = FileSystemMap::new();

        if let Some(pb) = &self.progress {
            pb.set_message("Starting filesystem scan...");
        }

        // Get directories to scan
        let scan_dirs = paths::get_user_scan_dirs()?;

        if let Some(pb) = &self.progress {
            pb.set_message(format!("Scanning {} directories...", scan_dirs.len()));
        }

        // Shared map for parallel processing
        let map_mutex = Arc::new(Mutex::new(map));

        // Process each scan directory
        for (idx, dir) in scan_dirs.iter().enumerate() {
            if !dir.exists() {
                continue;
            }

            if let Some(pb) = &self.progress {
                pb.set_message(format!(
                    "Scanning {}/{}: {}",
                    idx + 1,
                    scan_dirs.len(),
                    dir.display()
                ));
            }

            self.crawl_directory(dir, &map_mutex)?;
        }

        if let Some(pb) = &self.progress {
            pb.finish_with_message("Filesystem scan complete");
        }

        // Extract the final map
        let map = Arc::try_unwrap(map_mutex)
            .unwrap_or_else(|arc| Mutex::new(arc.lock().unwrap().clone()))
            .into_inner()
            .unwrap();

        Ok(map)
    }

    /// Crawl a specific directory using smart recursive approach
    fn crawl_directory(&self, dir: &Path, map: &Arc<Mutex<FileSystemMap>>) -> Result<()> {
        self.crawl_directory_recursive(dir, map, 0)
    }

    /// Recursive crawl with smart stopping
    fn crawl_directory_recursive(
        &self,
        dir: &Path,
        map: &Arc<Mutex<FileSystemMap>>,
        current_depth: usize,
    ) -> Result<()> {
        // Safety limit
        if current_depth > self.max_depth {
            return Ok(());
        }

        // Skip whitelisted directories
        if self.is_ignored(dir) {
            return Ok(());
        }

        // Skip protected directories
        if paths::is_protected_path(dir) {
            return Ok(());
        }

        // Classify and add ALL directories to the map
        if let Some(mut classified) = PathClassifier::classify(dir) {
            // Add all directories regardless of confidence
            {
                let mut map_lock = map.lock().unwrap();
                map_lock.add_directory(classified.clone());
            }

            // SMART STOP: Only stop descending into specific categories
            // Stop at build artifacts (we don't need to scan inside node_modules, target, etc.)
            if classified.confidence >= 0.8
                && matches!(classified.category, DirectoryCategory::BuildArtifact)
            {
                return Ok(());
            }
        }

        // Not classified or low confidence - continue searching deeper
        let entries: Vec<PathBuf> = match std::fs::read_dir(dir) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.path())
                .filter(|path| {
                    // Skip hidden dirs unless they might be interesting
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with('.') {
                            let name_lower = name.to_lowercase();
                            return [
                                "cache", "local", "config", "npm", "cargo", "gradle", "m2",
                                "flutter", "dart",
                            ]
                            .iter()
                            .any(|&s| name_lower.contains(s));
                        }
                    }
                    true
                })
                .collect(),
            Err(_) => return Ok(()), // Permission denied or other error
        };

        // Process subdirectories recursively
        for entry in entries {
            self.crawl_directory_recursive(&entry, map, current_depth + 1)?;
        }

        Ok(())
    }

    /// Incremental update: scan only specific directories
    pub fn update_directories(
        &self,
        directories: &[PathBuf],
        existing_map: &mut FileSystemMap,
    ) -> Result<()> {
        let map_mutex = Arc::new(Mutex::new(FileSystemMap::new()));

        for dir in directories {
            if dir.exists() {
                self.crawl_directory(dir, &map_mutex)?;
            }
        }

        // Merge with existing map
        let new_map = Arc::try_unwrap(map_mutex)
            .unwrap_or_else(|arc| Mutex::new(arc.lock().unwrap().clone()))
            .into_inner()
            .unwrap();

        existing_map.merge(new_map);

        Ok(())
    }

    /// Quick scan: only check common locations
    pub fn quick_scan(&self) -> Result<FileSystemMap> {
        let mut map = FileSystemMap::new();

        if let Some(pb) = &self.progress {
            pb.set_message("Quick scanning common locations...");
        }

        // Only scan well-known cache/temp locations
        if let Some(home) = dirs::home_dir() {
            let quick_dirs = match self.platform {
                Platform::MacOS => vec![home.join("Library/Caches"), home.join("Library/Logs")],
                Platform::Linux => vec![home.join(".cache"), home.join(".local/share")],
                Platform::Windows => vec![home.join("AppData\\Local\\Temp")],
            };

            for dir in quick_dirs {
                if !dir.exists() {
                    continue;
                }

                if let Some(classified) = PathClassifier::classify(&dir) {
                    if classified.confidence >= self.min_confidence {
                        map.add_directory(classified);
                    }
                }
            }
        }

        if let Some(pb) = &self.progress {
            pb.finish_with_message("Quick scan complete");
        }

        Ok(map)
    }

    /// Smart scan: use existing map as starting point and update stale entries
    pub fn smart_scan(&self, existing_map: &mut FileSystemMap) -> Result<()> {
        if let Some(pb) = &self.progress {
            pb.set_message("Smart scanning filesystem...");
        }

        // Get list of directories to re-check
        let stale_dirs: Vec<PathBuf> = existing_map
            .directories
            .values()
            .filter(|d| {
                // Re-check if directory doesn't exist or is likely to have changed (Ephemeral category)
                !d.path.exists() || matches!(d.category, DirectoryCategory::Ephemeral)
            })
            .map(|d| d.path.clone())
            .collect();

        // Remove non-existent directories
        existing_map.cleanup_invalid();

        // Update stale directories
        if !stale_dirs.is_empty() {
            self.update_directories(&stale_dirs, existing_map)?;
        }

        if let Some(pb) = &self.progress {
            pb.finish_with_message("Smart scan complete");
        }

        Ok(())
    }
}

impl Default for FileSystemCrawler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crawler_creation() {
        let crawler = FileSystemCrawler::new();
        assert_eq!(crawler.max_depth, 10);
        assert_eq!(crawler.min_confidence, 0.6);
    }

    #[test]
    fn test_crawler_configuration() {
        let crawler = FileSystemCrawler::new()
            .with_max_depth(5)
            .with_min_confidence(0.8);

        assert_eq!(crawler.max_depth, 5);
        assert_eq!(crawler.min_confidence, 0.8);
    }
}
