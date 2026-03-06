use std::path::PathBuf;
use anyhow::{Result, Context};

use super::Platform;

/// Get the configuration directory for the application
pub fn get_config_dir() -> Result<PathBuf> {
    let platform = Platform::current();

    match platform {
        Platform::MacOS => {
            if let Some(home) = dirs::home_dir() {
                Ok(home.join("Library/Application Support/cleanser"))
            } else {
                anyhow::bail!("Could not determine home directory")
            }
        }
        Platform::Linux => {
            if let Some(config_dir) = dirs::config_dir() {
                Ok(config_dir.join("cleanser"))
            } else if let Some(home) = dirs::home_dir() {
                Ok(home.join(".config/cleanser"))
            } else {
                anyhow::bail!("Could not determine config directory")
            }
        }
        Platform::Windows => {
            if let Some(config_dir) = dirs::config_dir() {
                Ok(config_dir.join("cleanser"))
            } else {
                anyhow::bail!("Could not determine config directory")
            }
        }
    }
}

/// Get the cache directory for the application
pub fn get_cache_dir() -> Result<PathBuf> {
    let platform = Platform::current();

    match platform {
        Platform::MacOS => {
            if let Some(home) = dirs::home_dir() {
                Ok(home.join("Library/Caches/cleanser"))
            } else {
                anyhow::bail!("Could not determine home directory")
            }
        }
        Platform::Linux => {
            if let Some(cache_dir) = dirs::cache_dir() {
                Ok(cache_dir.join("cleanser"))
            } else if let Some(home) = dirs::home_dir() {
                Ok(home.join(".cache/cleanser"))
            } else {
                anyhow::bail!("Could not determine cache directory")
            }
        }
        Platform::Windows => {
            if let Some(cache_dir) = dirs::cache_dir() {
                Ok(cache_dir.join("cleanser"))
            } else {
                anyhow::bail!("Could not determine cache directory")
            }
        }
    }
}

/// Get user home directory
pub fn get_home_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .context("Could not determine home directory")
}

/// Get common user directories to scan
pub fn get_user_scan_dirs() -> Result<Vec<PathBuf>> {
    let mut dirs = vec![];

    if let Some(home) = dirs::home_dir() {
        dirs.push(home.clone());

        // Add common subdirectories based on platform
        let platform = Platform::current();
        match platform {
            Platform::MacOS => {
                dirs.push(home.join("Library/Caches"));
                dirs.push(home.join("Library/Logs"));
                if let Some(downloads) = dirs::download_dir() {
                    dirs.push(downloads);
                }
            }
            Platform::Linux => {
                if let Some(cache) = dirs::cache_dir() {
                    dirs.push(cache);
                }
                if let Some(downloads) = dirs::download_dir() {
                    dirs.push(downloads);
                }
                dirs.push(home.join(".local/share"));
            }
            Platform::Windows => {
                if let Some(cache) = dirs::cache_dir() {
                    dirs.push(cache);
                }
                if let Some(downloads) = dirs::download_dir() {
                    dirs.push(downloads);
                }
                dirs.push(home.join("AppData\\Local\\Temp"));
            }
        }
    }

    Ok(dirs)
}

/// Get system-wide directories to scan (requires elevated privileges on some platforms)
pub fn get_system_scan_dirs() -> Vec<PathBuf> {
    let platform = Platform::current();

    match platform {
        Platform::MacOS => vec![
            PathBuf::from("/Library/Caches"),
            PathBuf::from("/Library/Logs"),
            PathBuf::from("/var/log"),
        ],
        Platform::Linux => vec![
            PathBuf::from("/var/log"),
            PathBuf::from("/var/cache"),
            PathBuf::from("/tmp"),
        ],
        Platform::Windows => vec![
            PathBuf::from("C:\\Windows\\Temp"),
        ],
    }
}

/// Check if a path should be protected from scanning
pub fn is_protected_path(path: &PathBuf) -> bool {
    let platform = Platform::current();
    let protected_dirs = platform.system_protected_dirs();

    for protected in protected_dirs {
        if path.starts_with(protected) {
            return true;
        }
    }

    false
}

/// Normalize a path to be absolute and canonical if possible
pub fn normalize_path(path: &PathBuf) -> PathBuf {
    // Try to canonicalize, but fall back to the original path if it fails
    path.canonicalize().unwrap_or_else(|_| path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_dir() {
        let config_dir = get_config_dir().unwrap();
        assert!(config_dir.to_string_lossy().contains("cleanser"));
    }

    #[test]
    fn test_get_cache_dir() {
        let cache_dir = get_cache_dir().unwrap();
        assert!(cache_dir.to_string_lossy().contains("cleanser"));
    }

    #[test]
    fn test_platform_detection() {
        let platform = Platform::current();
        assert!(matches!(platform, Platform::MacOS | Platform::Linux | Platform::Windows));
    }
}
