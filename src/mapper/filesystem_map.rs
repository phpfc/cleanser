use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::platform::paths;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DirectoryCategory {
    /// Temporary/cache data (safe to delete)
    Ephemeral,
    /// Build outputs (can be regenerated)
    BuildArtifact,
    /// Application/system data
    ApplicationData,
    /// User content
    UserContent,
    /// System files
    System,
    /// Unknown/unclassified
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappedDirectory {
    pub path: PathBuf,
    pub category: DirectoryCategory,
    /// Flexible tags for detailed classification
    /// Examples: ["cache", "npm"], ["docker", "vm"], ["xcode", "derived-data"]
    pub tags: Vec<String>,
    pub estimated_size: u64,
    pub file_count: usize,
    pub last_modified: Option<u64>,
    pub confidence: f32, // 0.0 to 1.0
}

impl MappedDirectory {
    /// Get primary tag (most relevant/first)
    pub fn primary_tag(&self) -> Option<&str> {
        self.tags.first().map(|s| s.as_str())
    }

    /// Check if directory has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Get human-readable description
    pub fn description(&self) -> String {
        let category_desc = match self.category {
            DirectoryCategory::Ephemeral => "Temporary/Cache",
            DirectoryCategory::BuildArtifact => "Build Output",
            DirectoryCategory::ApplicationData => "App Data",
            DirectoryCategory::UserContent => "User Content",
            DirectoryCategory::System => "System",
            DirectoryCategory::Unknown => "Unknown",
        };

        if let Some(tag) = self.primary_tag() {
            format!("{} ({})", category_desc, tag)
        } else {
            category_desc.to_string()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemMap {
    pub version: String,
    pub created_at: u64,
    pub last_updated: u64,
    pub directories: HashMap<PathBuf, MappedDirectory>,
    pub total_directories: usize,
}

impl FileSystemMap {
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: now,
            last_updated: now,
            directories: HashMap::new(),
            total_directories: 0,
        }
    }

    /// Add a mapped directory
    pub fn add_directory(&mut self, dir: MappedDirectory) {
        self.directories.insert(dir.path.clone(), dir);
        self.total_directories = self.directories.len();
        self.update_timestamp();
    }

    /// Get directories by category
    pub fn get_by_category(&self, category: DirectoryCategory) -> Vec<&MappedDirectory> {
        self.directories
            .values()
            .filter(|d| d.category == category)
            .collect()
    }

    /// Get directories by tag
    pub fn get_by_tag(&self, tag: &str) -> Vec<&MappedDirectory> {
        self.directories
            .values()
            .filter(|d| d.has_tag(tag))
            .collect()
    }

    /// Get directories with minimum confidence
    pub fn get_by_confidence(&self, min_confidence: f32) -> Vec<&MappedDirectory> {
        self.directories
            .values()
            .filter(|d| d.confidence >= min_confidence)
            .collect()
    }

    /// Load map from cache
    pub fn load() -> Result<Self> {
        let cache_dir = paths::get_cache_dir()?;
        let map_path = cache_dir.join("filesystem_map.json");

        if !map_path.exists() {
            return Ok(Self::new());
        }

        let contents = fs::read_to_string(&map_path)?;
        let map: FileSystemMap = serde_json::from_str(&contents)?;

        Ok(map)
    }

    /// Save map to cache
    pub fn save(&self) -> Result<()> {
        let cache_dir = paths::get_cache_dir()?;
        fs::create_dir_all(&cache_dir)?;

        let map_path = cache_dir.join("filesystem_map.json");
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&map_path, contents)?;

        Ok(())
    }

    /// Check if the map is stale (older than 7 days)
    pub fn is_stale(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let age_days = (now - self.last_updated) / 86400;
        age_days > 7
    }

    /// Update the last_updated timestamp
    fn update_timestamp(&mut self) {
        self.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Get total estimated size of all mapped directories
    pub fn total_size(&self) -> u64 {
        self.directories.values().map(|d| d.estimated_size).sum()
    }

    /// Get statistics by category
    pub fn stats_by_category(&self) -> HashMap<DirectoryCategory, (usize, u64)> {
        let mut stats: HashMap<DirectoryCategory, (usize, u64)> = HashMap::new();

        for dir in self.directories.values() {
            let entry = stats.entry(dir.category.clone()).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += dir.estimated_size;
        }

        stats
    }

    /// Get statistics by primary tag
    pub fn stats_by_tag(&self) -> HashMap<String, (usize, u64)> {
        let mut stats: HashMap<String, (usize, u64)> = HashMap::new();

        for dir in self.directories.values() {
            if let Some(tag) = dir.primary_tag() {
                let entry = stats.entry(tag.to_string()).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += dir.estimated_size;
            }
        }

        stats
    }

    /// Merge another map into this one, preferring newer entries
    pub fn merge(&mut self, other: FileSystemMap) {
        for (path, dir) in other.directories {
            if let Some(existing) = self.directories.get(&path) {
                // Keep the entry with higher confidence
                if dir.confidence > existing.confidence {
                    self.directories.insert(path, dir);
                }
            } else {
                self.directories.insert(path, dir);
            }
        }

        self.total_directories = self.directories.len();
        self.update_timestamp();
    }

    /// Remove directories that no longer exist
    pub fn cleanup_invalid(&mut self) {
        self.directories.retain(|path, _| path.exists());
        self.total_directories = self.directories.len();
        self.update_timestamp();
    }
}

impl Default for FileSystemMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_map() {
        let map = FileSystemMap::new();
        assert_eq!(map.total_directories, 0);
        assert_eq!(map.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_add_directory() {
        let mut map = FileSystemMap::new();
        let dir = MappedDirectory {
            path: PathBuf::from("/tmp/cache"),
            dir_type: DirectoryType::Cache,
            estimated_size: 1024,
            file_count: 10,
            last_modified: None,
            confidence: 0.9,
        };

        map.add_directory(dir);
        assert_eq!(map.total_directories, 1);
    }

    #[test]
    fn test_get_by_type() {
        let mut map = FileSystemMap::new();
        map.add_directory(MappedDirectory {
            path: PathBuf::from("/tmp/cache"),
            dir_type: DirectoryType::Cache,
            estimated_size: 1024,
            file_count: 10,
            last_modified: None,
            confidence: 0.9,
        });

        let caches = map.get_by_type(DirectoryType::Cache);
        assert_eq!(caches.len(), 1);
    }
}
