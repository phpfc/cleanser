use crate::types::ScanResults;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const CACHE_DIR: &str = ".cache/cleanser";
const CACHE_FILE: &str = "last-scan.json";
const CACHE_MAX_AGE_SECS: u64 = 3600; // 1 hour

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CachedScan {
    pub timestamp: u64,
    pub results: ScanResults,
}

/// Get the cache file path
fn get_cache_path() -> Result<PathBuf> {
    let home = std::env::var("HOME")?;
    Ok(PathBuf::from(home).join(CACHE_DIR).join(CACHE_FILE))
}

/// Save scan results to cache
pub fn save_scan_results(results: &ScanResults) -> Result<()> {
    let cache_path = get_cache_path()?;

    // Create cache directory if it doesn't exist
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();

    let cached = CachedScan {
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

    let cached: CachedScan = serde_json::from_str(&contents)
        .with_context(|| "Failed to parse cached scan results")?;

    let max_age = max_age_secs.unwrap_or(CACHE_MAX_AGE_SECS);
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();

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
    let cached: CachedScan = serde_json::from_str(&contents)?;

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();

    let age = current_time.saturating_sub(cached.timestamp);
    Ok(Some(age))
}
