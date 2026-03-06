use std::path::Path;
use crate::platform::detection::*;
use super::filesystem_map::{DirectoryCategory, MappedDirectory};

pub struct PathClassifier;

impl PathClassifier {
    /// Classify a directory and return a MappedDirectory with confidence score
    pub fn classify(path: &Path) -> Option<MappedDirectory> {
        // Get directory statistics first
        let stats = DirStats::analyze(path).ok()?;

        // Classify and get category + tags
        let (category, tags, confidence) = Self::classify_directory(path);

        Some(MappedDirectory {
            path: path.to_path_buf(),
            category,
            tags,
            estimated_size: stats.total_size,
            file_count: stats.file_count,
            last_modified: path
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
            confidence,
        })
    }

    /// Classify directory and return (category, tags, confidence)
    fn classify_directory(path: &Path) -> (DirectoryCategory, Vec<String>, f32) {
        let mut scores: Vec<(DirectoryType, f32)> = Vec::new();
        let path_str = path.to_string_lossy().to_lowercase();

        // High priority checks first (specific patterns)

        // Docker
        if Self::is_docker(path) {
            scores.push((DirectoryType::Docker, 0.99));
        }

        // VMs
        if Self::is_vm(path) {
            scores.push((DirectoryType::VM, 0.95));
        }

        // Xcode
        if Self::is_xcode(path) {
            scores.push((DirectoryType::Xcode, 0.95));
        }

        // User directories (Desktop, Documents, Downloads)
        if let Some((dir_type, conf)) = Self::is_user_directory(path) {
            scores.push((dir_type, conf));
        }

        // Containers
        if path_str.contains("/library/containers/") {
            scores.push((DirectoryType::Containers, 0.9));
        }

        // Application Support
        if path_str.contains("/library/application support/") ||
           path_str.contains("/.local/share/") {
            scores.push((DirectoryType::ApplicationSupport, 0.85));
        }

        // Check cache directory
        if is_likely_cache_dir(path) {
            let confidence = if Self::has_known_cache_parent(path) {
                0.95
            } else {
                0.8
            };
            scores.push((DirectoryType::Cache, confidence));
        }

        // Check log directory
        if is_likely_log_dir(path) {
            scores.push((DirectoryType::Log, 0.8));
        }

        // Check build directory
        if is_likely_build_dir(path) {
            let confidence = if Self::has_nearby_project_root(path) {
                0.95
            } else {
                0.7
            };
            scores.push((DirectoryType::Build, confidence));
        }

        // Check node_modules
        if is_node_modules(path) {
            let confidence = if Self::has_nearby_project_root(path) {
                0.99
            } else {
                0.8
            };
            scores.push((DirectoryType::PackageManager, confidence));
        }

        // Check for package manager specific caches
        if Self::is_package_manager_cache(path) {
            scores.push((DirectoryType::PackageManager, 0.92));
        }

        // Check for browser cache patterns
        if Self::is_browser_cache(path) {
            scores.push((DirectoryType::Browser, 0.9));
        }

        // Check for IDE
        if Self::is_ide_cache(path) {
            scores.push((DirectoryType::IDE, 0.88));
        }

        // Check for temp directory
        if Self::is_temp_dir(path) {
            scores.push((DirectoryType::Temp, 0.7));
        }

        // Media directories
        if Self::is_media(path) {
            scores.push((DirectoryType::Media, 0.75));
        }

        // Return the classification with highest confidence
        scores.into_iter().max_by(|a, b| {
            a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Check if there's a project root nearby (parent directories)
    fn has_nearby_project_root(path: &Path) -> bool {
        let mut current = path;

        for _ in 0..3 {
            // Check up to 3 levels up
            if let Some(parent) = current.parent() {
                if has_project_indicator(parent) {
                    return true;
                }
                current = parent;
            } else {
                break;
            }
        }

        false
    }

    /// Check if path is a browser cache
    fn is_browser_cache(path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();

        let browser_patterns = vec![
            "chrome/cache",
            "firefox/cache",
            "safari/cache",
            "edge/cache",
            "google/chrome",
            "mozilla/firefox",
        ];

        browser_patterns
            .iter()
            .any(|pattern| path_str.contains(pattern))
    }

    /// Check if path is a temp directory
    fn is_temp_dir(path: &Path) -> bool {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_lowercase();

        let temp_names = vec!["tmp", "temp", "temporary", ".tmp"];
        temp_names.iter().any(|&temp| name == temp || name.contains(temp))
    }

    /// Check if path has a known cache parent directory
    fn has_known_cache_parent(path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();

        let known_parents = vec![
            "library/caches",
            ".cache",
            ".local/share",
            "appdata/local",
            "appdata/locallow",
        ];

        known_parents.iter().any(|&parent| path_str.contains(parent))
    }

    /// Check if path is a package manager cache
    fn is_package_manager_cache(path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();

        let pm_patterns = vec![
            ".cargo/registry",
            ".cargo/git",
            ".npm/_cacache",
            ".npm/_logs",
            ".pnpm-store",
            ".yarn/cache",
            ".m2/repository",
            ".gradle/caches",
            ".cache/pip",
            ".cache/yarn",
            ".cache/go-build",
            "library/caches/homebrew",
            "library/caches/pip",
            "caches/homebrew",
            ".cache/composer",
            ".bundle/cache",
        ];

        pm_patterns.iter().any(|&pattern| path_str.contains(pattern))
    }

    /// Check if path is an IDE cache directory
    fn is_ide_cache(path: &Path) -> bool {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_lowercase();

        let path_str = path.to_string_lossy().to_lowercase();

        // IDE-specific patterns
        let ide_patterns = vec![
            ".idea",
            ".vscode/extensions",
            ".vs/",
            "__pycache__",
            ".pytest_cache",
            ".mypy_cache",
            ".tox",
            ".nox",
            "target/debug",
            "target/release",
            ".dart_tool",
            ".flutter",
        ];

        ide_patterns.iter().any(|&pattern| {
            name.contains(pattern) || path_str.contains(pattern)
        })
    }

    /// Reclassify a directory with additional context
    pub fn reclassify_with_context(
        path: &Path,
        parent_type: Option<&DirectoryType>,
    ) -> Option<MappedDirectory> {
        let mut result = Self::classify(path)?;

        // Adjust confidence based on parent type
        if let Some(parent) = parent_type {
            match (parent, &result.dir_type) {
                // If parent is Cache and child is also Cache, increase confidence
                (DirectoryType::Cache, DirectoryType::Cache) => {
                    result.confidence = (result.confidence * 1.2).min(1.0);
                }
                // If parent is Build and child is Cache, it's likely build cache
                (DirectoryType::Build, DirectoryType::Cache) => {
                    result.dir_type = DirectoryType::Build;
                    result.confidence = 0.95;
                }
                _ => {}
            }
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_classify_cache() {
        let path = PathBuf::from("/home/user/.cache");
        if let Some(classified) = PathClassifier::classify(&path) {
            assert_eq!(classified.dir_type, DirectoryType::Cache);
            assert!(classified.confidence > 0.5);
        }
    }

    #[test]
    fn test_classify_build() {
        let path = PathBuf::from("/project/target");
        if let Some(classified) = PathClassifier::classify(&path) {
            assert_eq!(classified.dir_type, DirectoryType::Build);
        }
    }

    #[test]
    fn test_browser_cache_detection() {
        assert!(PathClassifier::is_browser_cache(
            &PathBuf::from("/home/user/.cache/google/chrome")
        ));
        assert!(!PathClassifier::is_browser_cache(
            &PathBuf::from("/home/user/documents")
        ));
    }
}
