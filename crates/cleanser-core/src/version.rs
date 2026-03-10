//! Version checking module for Cleanser
//!
//! This module provides functionality to check for new versions of Cleanser
//! by querying the GitHub releases API.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Current version of Cleanser
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Cache duration for version checks (24 hours)
const CACHE_DURATION: Duration = Duration::from_secs(24 * 60 * 60);

/// GitHub API URL for latest release
const GITHUB_RELEASES_URL: &str = "https://api.github.com/repos/phpfc/cleanser/releases/latest";

/// Information about available versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Current installed version
    pub current: String,
    /// Latest available version (if check succeeded)
    pub latest: Option<String>,
    /// Whether an update is available
    pub update_available: bool,
    /// URL to the release page
    pub release_url: Option<String>,
    /// Release notes/description
    pub release_notes: Option<String>,
}

impl Default for VersionInfo {
    fn default() -> Self {
        Self {
            current: CURRENT_VERSION.to_string(),
            latest: None,
            update_available: false,
            release_url: None,
            release_notes: None,
        }
    }
}

/// GitHub release response structure
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
}

/// Cached version check result
#[derive(Debug, Serialize, Deserialize)]
struct VersionCache {
    checked_at: u64,
    info: VersionInfo,
}

/// Get the path to the version cache file
fn get_cache_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|p| p.join("cleanser").join("version_cache.json"))
}

/// Load cached version info if still valid
fn load_cached_version() -> Option<VersionInfo> {
    let cache_path = get_cache_path()?;

    let content = fs::read_to_string(&cache_path).ok()?;
    let cache: VersionCache = serde_json::from_str(&content).ok()?;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_secs();

    if now - cache.checked_at < CACHE_DURATION.as_secs() {
        Some(cache.info)
    } else {
        None
    }
}

/// Save version info to cache
fn save_to_cache(info: &VersionInfo) -> Result<()> {
    let cache_path = match get_cache_path() {
        Some(p) => p,
        None => return Ok(()),
    };

    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    let cache = VersionCache {
        checked_at: now,
        info: info.clone(),
    };

    fs::write(&cache_path, serde_json::to_string_pretty(&cache)?)?;
    Ok(())
}

/// Compare two version strings using semver
#[cfg(feature = "update-check")]
fn is_newer_version(latest: &str, current: &str) -> bool {
    let latest_clean = latest.trim_start_matches('v');
    let current_clean = current.trim_start_matches('v');

    match (
        semver::Version::parse(latest_clean),
        semver::Version::parse(current_clean),
    ) {
        (Ok(latest_ver), Ok(current_ver)) => latest_ver > current_ver,
        _ => false,
    }
}

/// Check for updates asynchronously
///
/// This function queries the GitHub releases API to check if a newer version
/// of Cleanser is available. Results are cached for 24 hours.
///
/// # Returns
///
/// Returns a `VersionInfo` struct with information about the current and latest versions.
/// If the check fails (e.g., no internet), it returns the current version info with
/// `update_available` set to false.
///
/// # Example
///
/// ```ignore
/// let info = cleanser_core::version::check_for_updates().await?;
/// if info.update_available {
///     println!("New version available: {}", info.latest.unwrap());
/// }
/// ```
#[cfg(feature = "update-check")]
pub async fn check_for_updates() -> Result<VersionInfo> {
    // Check cache first
    if let Some(cached) = load_cached_version() {
        return Ok(cached);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let response = client
        .get(GITHUB_RELEASES_URL)
        .header("User-Agent", format!("cleanser/{}", CURRENT_VERSION))
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(VersionInfo::default());
    }

    let release: GitHubRelease = response.json().await?;
    let latest = release.tag_name.trim_start_matches('v').to_string();

    let update_available = is_newer_version(&latest, CURRENT_VERSION);

    let info = VersionInfo {
        current: CURRENT_VERSION.to_string(),
        latest: Some(latest),
        update_available,
        release_url: Some(release.html_url),
        release_notes: release.body,
    };

    // Cache the result
    let _ = save_to_cache(&info);

    Ok(info)
}

/// Check for updates synchronously (blocking)
///
/// This is a convenience wrapper around `check_for_updates` for use in
/// synchronous contexts. It spawns a tokio runtime internally.
#[cfg(feature = "update-check")]
pub fn check_for_updates_sync() -> Result<VersionInfo> {
    // Check cache first (no need for async)
    if let Some(cached) = load_cached_version() {
        return Ok(cached);
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(check_for_updates())
}

/// Check for updates in a background thread
///
/// This function spawns a background thread to check for updates without
/// blocking the main application. The callback is called with the result.
#[cfg(feature = "update-check")]
pub fn check_for_updates_background<F>(callback: F)
where
    F: FnOnce(Result<VersionInfo>) + Send + 'static,
{
    std::thread::spawn(move || {
        let result = check_for_updates_sync();
        callback(result);
    });
}

/// Clear the version cache
pub fn clear_version_cache() -> Result<()> {
    if let Some(cache_path) = get_cache_path() {
        if cache_path.exists() {
            fs::remove_file(&cache_path)?;
        }
    }
    Ok(())
}

/// Get the current version string
pub fn current_version() -> &'static str {
    CURRENT_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_version_format() {
        let version = current_version();
        assert!(!version.is_empty());
        // Should be valid semver
        assert!(semver::Version::parse(version).is_ok());
    }

    #[test]
    fn test_version_info_default() {
        let info = VersionInfo::default();
        assert_eq!(info.current, CURRENT_VERSION);
        assert!(!info.update_available);
        assert!(info.latest.is_none());
    }

    #[cfg(feature = "update-check")]
    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("1.0.0", "0.9.0"));
        assert!(is_newer_version("v1.0.0", "0.9.0"));
        assert!(is_newer_version("0.6.0", "0.5.2"));
        assert!(!is_newer_version("0.5.2", "0.5.2"));
        assert!(!is_newer_version("0.5.0", "0.5.2"));
    }
}
