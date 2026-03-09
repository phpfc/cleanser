use crate::platform;
use crate::types::ScanResults;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const CACHE_DIR: &str = ".cache/cleanser";
const CACHE_FILE: &str = "last-scan.json";
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

/// Save scan results to cache
pub fn save_scan_results(results: &ScanResults) -> Result<()> {
    let cache_path = get_cache_path()?;

    // Create cache directory if it doesn't exist
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let cached = CachedScan {
        version: CACHE_VERSION,
        timestamp,
        results: results.clone(),
    };

    let json = serde_json::to_string_pretty(&cached)?;
    fs::write(&cache_path, json)
        .with_context(|| format!("Failed to write cache to {:?}", cache_path))?;

    Ok(())
}

/// Load scan results from cache if they exist and are fresh
pub fn load_scan_results(max_age_secs: Option<u64>) -> Result<Option<ScanResults>> {
    let cache_path = get_cache_path()?;

    if !cache_path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&cache_path)
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
#[allow(dead_code)]
pub fn clear_cache() -> Result<()> {
    let cache_path = get_cache_path()?;

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

    let contents = fs::read_to_string(&cache_path)?;
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
pub fn update_cache_after_deletion(deleted_paths: &[PathBuf]) -> Result<()> {
    use crate::types::CleanableItem;

    let cache_path = get_cache_path()?;

    if !cache_path.exists() {
        // No cache to update
        return Ok(());
    }

    let contents = fs::read_to_string(&cache_path)?;
    let mut cached: CachedScan = match serde_json::from_str(&contents) {
        Ok(c) => c,
        Err(_) => return Ok(()),
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

    // Save updated cache
    let json = serde_json::to_string_pretty(&cached)?;
    fs::write(&cache_path, json)
        .with_context(|| format!("Failed to update cache at {:?}", cache_path))?;

    if removed_count > 0 {
        println!("Cache updated: removed {} deleted items", removed_count);
    }

    Ok(())
}
