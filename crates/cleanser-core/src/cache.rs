//! Scan result caching.
//!
//! This module handles caching of scan results to avoid re-scanning
//! when the user wants to clean recently scanned items.
//!
//! ## File Locking
//!
//! This module uses file locking to prevent race conditions when multiple
//! instances of cleanser run simultaneously. All read and write operations
//! acquire appropriate locks before accessing the cache file.

use crate::platform;
use crate::types::ScanResults;
use anyhow::{Context, Result};
use fs2::FileExt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const CACHE_DIR: &str = ".cache/cleanser";
const CACHE_FILE: &str = "last-scan.json";
const LOCK_FILE: &str = "last-scan.lock";
const CACHE_MAX_AGE_SECS: u64 = 3600; // 1 hour
const CACHE_VERSION: u32 = 1;

fn default_version() -> u32 {
    0
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CachedScan {
    #[serde(default = "default_version")]
    pub version: u32,
    pub timestamp: u64,
    pub results: ScanResults,
}

/// Get the cache file path
fn get_cache_path() -> Result<PathBuf> {
    let home = platform::home_dir_or_err()?;
    Ok(home.join(CACHE_DIR).join(CACHE_FILE))
}

/// Get the lock file path
fn get_lock_path() -> Result<PathBuf> {
    let home = platform::home_dir_or_err()?;
    Ok(home.join(CACHE_DIR).join(LOCK_FILE))
}

/// Ensure the cache directory exists
fn ensure_cache_dir() -> Result<PathBuf> {
    let home = platform::home_dir_or_err()?;
    let cache_dir = home.join(CACHE_DIR);
    fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

/// RAII guard for file lock
struct FileLockGuard {
    file: File,
}

impl FileLockGuard {
    /// Acquire an exclusive lock for writing
    fn exclusive(path: &PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .with_context(|| format!("Failed to open lock file {:?}", path))?;

        file.lock_exclusive()
            .with_context(|| "Failed to acquire exclusive lock")?;

        Ok(Self { file })
    }

    /// Acquire a shared lock for reading
    fn shared(path: &PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .with_context(|| format!("Failed to open lock file {:?}", path))?;

        file.lock_shared()
            .with_context(|| "Failed to acquire shared lock")?;

        Ok(Self { file })
    }
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// Save scan results to cache
pub fn save_scan_results(results: &ScanResults) -> Result<()> {
    ensure_cache_dir()?;
    let cache_path = get_cache_path()?;
    let lock_path = get_lock_path()?;

    // Acquire exclusive lock
    let _lock = FileLockGuard::exclusive(&lock_path)?;

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let cached = CachedScan {
        version: CACHE_VERSION,
        timestamp,
        results: results.clone(),
    };

    let json = serde_json::to_string_pretty(&cached)?;

    // Write to temp file first, then rename for atomicity
    let temp_path = cache_path.with_extension("tmp");
    let mut file = File::create(&temp_path)
        .with_context(|| format!("Failed to create temp cache file {:?}", temp_path))?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?;

    fs::rename(&temp_path, &cache_path)
        .with_context(|| format!("Failed to rename cache file to {:?}", cache_path))?;

    Ok(())
}

/// Load scan results from cache if they exist and are fresh
pub fn load_scan_results(max_age_secs: Option<u64>) -> Result<Option<ScanResults>> {
    let cache_path = get_cache_path()?;

    if !cache_path.exists() {
        return Ok(None);
    }

    let lock_path = get_lock_path()?;

    // Acquire shared lock for reading
    let _lock = FileLockGuard::shared(&lock_path)?;

    let mut file = match File::open(&cache_path) {
        Ok(f) => f,
        Err(_) => return Ok(None),
    };

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .with_context(|| format!("Failed to read cache from {:?}", cache_path))?;

    let cached: CachedScan = match serde_json::from_str(&contents) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };

    if cached.version != CACHE_VERSION {
        return Ok(None);
    }

    let max_age = max_age_secs.unwrap_or(CACHE_MAX_AGE_SECS);
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let age = current_time.saturating_sub(cached.timestamp);

    if age > max_age {
        // Cache is too old
        return Ok(None);
    }

    Ok(Some(cached.results))
}

/// Clear the scan cache
pub fn clear_cache() -> Result<()> {
    let cache_path = get_cache_path()?;

    if !cache_path.exists() {
        return Ok(());
    }

    let lock_path = get_lock_path()?;

    // Acquire exclusive lock
    let _lock = FileLockGuard::exclusive(&lock_path)?;

    if cache_path.exists() {
        fs::remove_file(&cache_path)
            .with_context(|| format!("Failed to remove cache file {:?}", cache_path))?;
    }

    Ok(())
}

/// Get cache age in seconds, or None if no cache exists
pub fn get_cache_age() -> Result<Option<u64>> {
    let cache_path = get_cache_path()?;

    if !cache_path.exists() {
        return Ok(None);
    }

    let lock_path = get_lock_path()?;

    // Acquire shared lock for reading
    let _lock = FileLockGuard::shared(&lock_path)?;

    let contents = match fs::read_to_string(&cache_path) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };

    let cached: CachedScan = match serde_json::from_str(&contents) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };

    if cached.version != CACHE_VERSION {
        return Ok(None);
    }

    let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let age = current_time.saturating_sub(cached.timestamp);
    Ok(Some(age))
}

/// Update cache by removing deleted items
/// Returns the number of items removed from cache
pub fn update_cache_after_deletion(deleted_paths: &[PathBuf]) -> Result<usize> {
    use crate::types::CleanableItem;

    let cache_path = get_cache_path()?;

    if !cache_path.exists() {
        // No cache to update
        return Ok(0);
    }

    let lock_path = get_lock_path()?;

    // Acquire exclusive lock for read-modify-write
    let _lock = FileLockGuard::exclusive(&lock_path)?;

    let contents = match fs::read_to_string(&cache_path) {
        Ok(c) => c,
        Err(_) => return Ok(0),
    };

    let mut cached: CachedScan = match serde_json::from_str(&contents) {
        Ok(c) => c,
        Err(_) => return Ok(0),
    };

    // Ensure version is current when saving
    cached.version = CACHE_VERSION;

    // Remove deleted items from cache
    let original_count = cached.results.items.len();
    cached.results.items.retain(|item: &CleanableItem| {
        !deleted_paths
            .iter()
            .any(|deleted_path| deleted_path == &item.path)
    });

    let removed_count = original_count - cached.results.items.len();

    // Recalculate total size
    cached.results.total_size = cached.results.items.iter().map(|item| item.size).sum();

    // Update timestamp
    cached.timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    // Save updated cache atomically
    let json = serde_json::to_string_pretty(&cached)?;
    let temp_path = cache_path.with_extension("tmp");
    let mut file = File::create(&temp_path)?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?;

    fs::rename(&temp_path, &cache_path)
        .with_context(|| format!("Failed to update cache at {:?}", cache_path))?;

    Ok(removed_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_guard_exclusive() {
        let temp_dir = tempfile::tempdir().unwrap();
        let lock_path = temp_dir.path().join("test.lock");

        let _guard = FileLockGuard::exclusive(&lock_path).unwrap();
        assert!(lock_path.exists());
    }

    #[test]
    fn test_lock_guard_shared() {
        let temp_dir = tempfile::tempdir().unwrap();
        let lock_path = temp_dir.path().join("test.lock");

        let _guard = FileLockGuard::shared(&lock_path).unwrap();
        assert!(lock_path.exists());
    }
}
