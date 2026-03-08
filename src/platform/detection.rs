#![allow(dead_code)]

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use super::Platform;

/// Detect if a directory is likely a cache directory based on heuristics
pub fn is_likely_cache_dir(path: &Path) -> bool {
    let name_lower = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Check common cache directory names
    let cache_names = [
        "cache",
        "caches",
        "tmp",
        "temp",
        "temporary",
        ".cache",
        "_cache",
        "cacache",
    ];

    cache_names.iter().any(|&cache| name_lower.contains(cache))
}

/// Detect if a directory is likely a log directory
pub fn is_likely_log_dir(path: &Path) -> bool {
    let name_lower = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let log_names = ["log", "logs", ".log", "_logs"];

    log_names.iter().any(|&log| name_lower.contains(log))
}

/// Detect if a file is likely a log file
pub fn is_likely_log_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        if ext_lower == "log" {
            return true;
        }
    }

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_lowercase();

    name.contains(".log") || name.ends_with(".log")
}

/// Detect if a directory is a build artifact directory
pub fn is_likely_build_dir(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();

    let build_names = [
        "target",
        "build",
        "dist",
        "out",
        "_build",
        ".next",
        ".nuxt",
        "__pycache__",
    ];

    build_names.contains(&name)
}

/// Detect if a directory is node_modules
pub fn is_node_modules(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n == "node_modules")
        .unwrap_or(false)
}

/// Detect if a path contains a project root indicator
pub fn has_project_indicator(dir: &Path) -> bool {
    let indicators = [
        "package.json",
        "Cargo.toml",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "pyproject.toml",
        "setup.py",
        ".git",
    ];

    indicators
        .iter()
        .any(|&indicator| dir.join(indicator).exists())
}

/// Detect common package managers installed on the system
pub fn detect_package_managers() -> Vec<String> {
    let mut managers = Vec::new();

    // Check for common package managers in PATH
    let pm_commands = [
        ("npm", "npm"),
        ("cargo", "cargo"),
        ("pip", "pip"),
        ("brew", "homebrew"),
        ("apt", "apt"),
        ("dnf", "dnf"),
        ("pacman", "pacman"),
        ("yarn", "yarn"),
        ("pnpm", "pnpm"),
    ];

    for (cmd, name) in pm_commands {
        if which_exists(cmd) {
            managers.push(name.to_string());
        }
    }

    managers
}

/// Simple check if a command exists in PATH
fn which_exists(command: &str) -> bool {
    std::process::Command::new(if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    })
    .arg(command)
    .output()
    .map(|output| output.status.success())
    .unwrap_or(false)
}

/// Detect browser cache directories
pub fn detect_browser_caches(home: &Path) -> Vec<PathBuf> {
    let platform = Platform::current();
    let mut caches = Vec::new();

    match platform {
        Platform::MacOS => {
            let browsers = vec![
                "Library/Caches/Google/Chrome",
                "Library/Caches/Firefox",
                "Library/Caches/com.apple.Safari",
                "Library/Caches/Microsoft Edge",
            ];

            for browser in browsers {
                let path = home.join(browser);
                if path.exists() {
                    caches.push(path);
                }
            }
        }
        Platform::Linux => {
            let browsers = vec![
                ".cache/google-chrome",
                ".cache/mozilla/firefox",
                ".cache/chromium",
            ];

            for browser in browsers {
                let path = home.join(browser);
                if path.exists() {
                    caches.push(path);
                }
            }
        }
        Platform::Windows => {
            let browsers = vec![
                "AppData\\Local\\Google\\Chrome\\User Data\\Default\\Cache",
                "AppData\\Local\\Microsoft\\Edge\\User Data\\Default\\Cache",
            ];

            for browser in browsers {
                let path = home.join(browser);
                if path.exists() {
                    caches.push(path);
                }
            }
        }
    }

    caches
}

/// Calculate directory statistics for heuristics
#[derive(Debug)]
pub struct DirStats {
    pub file_count: usize,
    pub total_size: u64,
    pub avg_file_size: u64,
}

impl DirStats {
    pub fn analyze(path: &Path) -> Result<Self> {
        let mut file_count = 0;
        let mut total_size = 0u64;

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        file_count += 1;
                        total_size += metadata.len();
                    }
                }
            }
        }

        let avg_file_size = if file_count > 0 {
            total_size / file_count as u64
        } else {
            0
        };

        Ok(DirStats {
            file_count,
            total_size,
            avg_file_size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_dir_detection() {
        assert!(is_likely_cache_dir(Path::new("/tmp/cache")));
        assert!(is_likely_cache_dir(Path::new("/home/user/.cache")));
        assert!(is_likely_cache_dir(Path::new("/Library/Caches")));
        assert!(!is_likely_cache_dir(Path::new("/home/user/documents")));
    }

    #[test]
    fn test_log_detection() {
        assert!(is_likely_log_file(Path::new("/var/log/system.log")));
        assert!(is_likely_log_file(Path::new("error.log")));
        assert!(!is_likely_log_file(Path::new("readme.txt")));
    }

    #[test]
    fn test_build_dir_detection() {
        assert!(is_likely_build_dir(Path::new("/project/target")));
        assert!(is_likely_build_dir(Path::new("/project/dist")));
        assert!(!is_likely_build_dir(Path::new("/project/src")));
    }

    #[test]
    fn test_node_modules_detection() {
        assert!(is_node_modules(Path::new("/project/node_modules")));
        assert!(!is_node_modules(Path::new("/project/src")));
    }
}
