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

    /// Main classification logic - returns (category, tags, confidence)
    fn classify_directory(path: &Path) -> (DirectoryCategory, Vec<String>, f32) {
        let path_str = path.to_string_lossy().to_lowercase();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        let mut tags = Vec::new();
        let mut confidence = 0.3; // Default low confidence
        let mut category = DirectoryCategory::Unknown;

        // Build artifacts - highest priority
        if is_node_modules(path) {
            category = DirectoryCategory::BuildArtifact;
            tags.push("node_modules".to_string());
            confidence = if has_project_indicator(path.parent().unwrap_or(path)) {
                0.99
            } else {
                0.85
            };
        } else if is_likely_build_dir(path) {
            category = DirectoryCategory::BuildArtifact;
            tags.push(name.clone());
            if Self::has_nearby_project_root(path) {
                confidence = 0.95;
            } else {
                confidence = 0.75;
            }
        }

        // Ephemeral (caches, logs, temp)
        else if Self::is_package_manager_cache(path) {
            category = DirectoryCategory::Ephemeral;
            Self::extract_pm_tags(&path_str, &mut tags);
            tags.push("cache".to_string());
            confidence = 0.92;
        } else if is_likely_cache_dir(path) {
            category = DirectoryCategory::Ephemeral;
            tags.push("cache".to_string());
            confidence = if Self::has_known_cache_parent(path) { 0.95 } else { 0.80 };
        } else if is_likely_log_dir(path) {
            category = DirectoryCategory::Ephemeral;
            tags.push("log".to_string());
            confidence = 0.80;
        } else if Self::is_temp_dir(path) {
            category = DirectoryCategory::Ephemeral;
            tags.push("temp".to_string());
            confidence = 0.70;
        }

        // Application Data
        else if path_str.contains("docker") && (path_str.contains("/vms/") || path_str.contains("/data/")) {
            category = DirectoryCategory::ApplicationData;
            tags.push("docker".to_string());
            if path_str.contains("/vms/") {
                tags.push("vm".to_string());
            }
            confidence = 0.95;
        } else if path_str.contains("xcode") && path_str.contains("deriveddata") {
            category = DirectoryCategory::ApplicationData;
            tags.push("xcode".to_string());
            tags.push("derived-data".to_string());
            confidence = 0.95;
        } else if path_str.contains("/library/containers/") {
            category = DirectoryCategory::ApplicationData;
            tags.push("container".to_string());
            confidence = 0.90;
        } else if path_str.contains("/library/application support/") ||
                  path_str.contains("/.local/share/") {
            category = DirectoryCategory::ApplicationData;
            tags.push("app-support".to_string());
            confidence = 0.75;
        }

        // User Content
        else if name == "downloads" || path_str.ends_with("/downloads") {
            category = DirectoryCategory::UserContent;
            tags.push("downloads".to_string());
            confidence = 0.95;
        } else if name == "documents" || path_str.ends_with("/documents") {
            category = DirectoryCategory::UserContent;
            tags.push("documents".to_string());
            confidence = 0.95;
        } else if name == "desktop" || path_str.ends_with("/desktop") {
            category = DirectoryCategory::UserContent;
            tags.push("desktop".to_string());
            confidence = 0.95;
        } else if Self::is_media_dir(path) {
            category = DirectoryCategory::UserContent;
            tags.push("media".to_string());
            confidence = 0.70;
        }

        // System
        else if path_str.starts_with("/system") ||
                path_str.starts_with("/library") && !path_str.contains("caches") {
            category = DirectoryCategory::System;
            tags.push("system".to_string());
            confidence = 0.85;
        }

        // Extract additional contextual tags
        Self::add_contextual_tags(path, &mut tags);

        (category, tags, confidence)
    }

    /// Extract package manager tags from path
    fn extract_pm_tags(path_str: &str, tags: &mut Vec<String>) {
        if path_str.contains(".cargo") {
            tags.push("cargo".to_string());
        }
        if path_str.contains(".npm") || path_str.contains("node_modules") {
            tags.push("npm".to_string());
        }
        if path_str.contains("pip") || path_str.contains("__pycache__") {
            tags.push("pip".to_string());
        }
        if path_str.contains("homebrew") {
            tags.push("homebrew".to_string());
        }
        if path_str.contains(".gradle") {
            tags.push("gradle".to_string());
        }
        if path_str.contains(".m2") {
            tags.push("maven".to_string());
        }
        if path_str.contains("yarn") {
            tags.push("yarn".to_string());
        }
        if path_str.contains("pnpm") {
            tags.push("pnpm".to_string());
        }
    }

    /// Add contextual tags based on parent directories or file presence
    fn add_contextual_tags(path: &Path, tags: &mut Vec<String>) {
        let path_str = path.to_string_lossy().to_lowercase();

        // IDE tags
        if path_str.contains(".idea") {
            tags.push("intellij".to_string());
        }
        if path_str.contains(".vscode") {
            tags.push("vscode".to_string());
        }

        // Language/framework tags
        if path_str.contains("flutter") || path_str.contains(".flutter") {
            tags.push("flutter".to_string());
        }
        if path_str.contains("dart") {
            tags.push("dart".to_string());
        }
        if path_str.contains("python") {
            tags.push("python".to_string());
        }
        if path_str.contains("rust") {
            tags.push("rust".to_string());
        }
    }

    fn has_nearby_project_root(path: &Path) -> bool {
        let mut current = path;
        for _ in 0..3 {
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

    fn has_known_cache_parent(path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        let known_parents = vec![
            "library/caches",
            ".cache",
            ".local/share",
            "appdata/local",
        ];
        known_parents.iter().any(|&parent| path_str.contains(parent))
    }

    fn is_package_manager_cache(path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        let pm_patterns = vec![
            ".cargo/registry",
            ".cargo/git",
            ".npm/_cacache",
            ".pnpm-store",
            ".yarn/cache",
            ".m2/repository",
            ".gradle/caches",
            ".cache/pip",
            "homebrew",
        ];
        pm_patterns.iter().any(|&pattern| path_str.contains(pattern))
    }

    fn is_temp_dir(path: &Path) -> bool {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_lowercase();
        let temp_names = vec!["tmp", "temp", "temporary"];
        temp_names.iter().any(|&temp| name == temp)
    }

    fn is_media_dir(path: &Path) -> bool {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_lowercase();
        let media_names = vec!["music", "videos", "pictures", "photos", "movies"];
        media_names.iter().any(|&m| name == m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_classify_node_modules() {
        let path = PathBuf::from("/project/node_modules");
        let (category, tags, conf) = PathClassifier::classify_directory(&path);
        assert_eq!(category, DirectoryCategory::BuildArtifact);
        assert!(tags.contains(&"node_modules".to_string()));
        assert!(conf > 0.8);
    }

    #[test]
    fn test_classify_cache() {
        let path = PathBuf::from("/home/user/.cache/pip");
        let (category, tags, conf) = PathClassifier::classify_directory(&path);
        assert_eq!(category, DirectoryCategory::Ephemeral);
        assert!(tags.contains(&"cache".to_string()));
        assert!(tags.contains(&"pip".to_string()));
    }
}
