//! Trash-based deletion for file recovery.
//!
//! This module provides trash functionality that moves files to a trash
//! location instead of permanently deleting them, allowing for recovery.

use super::journal::{TrashEntry, TrashJournal};
use crate::platform::Platform;
use crate::types::CleanCategory;
use crate::utils::get_dir_size;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Configuration for trash behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashConfig {
    /// Whether to use system trash (not yet implemented, always uses custom)
    pub use_system_trash: bool,
    /// Custom trash directory path (overrides default)
    pub custom_trash_path: Option<PathBuf>,
    /// Auto-empty trash after this many days (None = never)
    pub auto_empty_days: Option<u32>,
    /// Maximum trash size in bytes (None = unlimited)
    pub max_trash_size: Option<u64>,
}

impl Default for TrashConfig {
    fn default() -> Self {
        Self {
            use_system_trash: false, // Use cleanser's own trash for now
            custom_trash_path: None,
            auto_empty_days: Some(30),
            max_trash_size: None,
        }
    }
}

/// Trash manager for moving files to trash and restoring them
pub struct TrashManager {
    config: TrashConfig,
    journal: TrashJournal,
    trash_dir: PathBuf,
    /// If true, don't persist journal to disk (for testing)
    #[cfg(test)]
    skip_persist: bool,
}

impl TrashManager {
    /// Create a new trash manager with the given configuration
    pub fn new(config: TrashConfig) -> Result<Self> {
        let trash_dir = Self::get_trash_directory(&config)?;

        // Ensure trash directory exists
        fs::create_dir_all(&trash_dir)
            .with_context(|| format!("Failed to create trash directory: {}", trash_dir.display()))?;

        let journal = TrashJournal::load()?;

        Ok(Self {
            config,
            journal,
            trash_dir,
            #[cfg(test)]
            skip_persist: false,
        })
    }

    /// Create a trash manager with an empty in-memory journal (for testing)
    #[cfg(test)]
    fn new_isolated(config: TrashConfig) -> Result<Self> {
        let trash_dir = Self::get_trash_directory(&config)?;

        fs::create_dir_all(&trash_dir)
            .with_context(|| format!("Failed to create trash directory: {}", trash_dir.display()))?;

        // Use a fresh, empty journal that doesn't load from disk
        let journal = TrashJournal::default();

        Ok(Self {
            config,
            journal,
            trash_dir,
            skip_persist: true,
        })
    }

    /// Save journal if persistence is enabled
    fn save_journal(&self) -> Result<()> {
        #[cfg(test)]
        if self.skip_persist {
            return Ok(());
        }
        self.journal.save()
    }

    /// Create a new trash manager with default configuration
    pub fn with_defaults() -> Result<Self> {
        Self::new(TrashConfig::default())
    }

    /// Get the trash directory based on platform and config
    fn get_trash_directory(config: &TrashConfig) -> Result<PathBuf> {
        if let Some(ref custom_path) = config.custom_trash_path {
            return Ok(custom_path.clone());
        }

        let trash_dir = match Platform::current() {
            Platform::MacOS => {
                // Use ~/.cleanser-trash instead of system trash
                dirs::home_dir()
                    .map(|p| p.join(".cleanser-trash"))
                    .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
            }
            Platform::Linux => {
                // freedesktop.org spec: ~/.local/share/Trash
                // But we use our own to avoid conflicts
                dirs::data_local_dir()
                    .map(|p| p.join("cleanser-trash"))
                    .or_else(|| dirs::home_dir().map(|p| p.join(".cleanser-trash")))
                    .ok_or_else(|| anyhow::anyhow!("Could not find data directory"))?
            }
            Platform::Windows => {
                // Use our own trash location
                dirs::data_local_dir()
                    .map(|p| p.join("cleanser-trash"))
                    .or_else(|| dirs::home_dir().map(|p| p.join(".cleanser-trash")))
                    .ok_or_else(|| anyhow::anyhow!("Could not find data directory"))?
            }
        };

        Ok(trash_dir)
    }

    /// Get the trash directory path
    pub fn trash_dir(&self) -> &Path {
        &self.trash_dir
    }

    /// Move a file or directory to trash
    pub fn trash(&mut self, path: &Path) -> Result<TrashEntry> {
        self.trash_with_info(path, None, None)
    }

    /// Move a file or directory to trash with additional info
    pub fn trash_with_info(
        &mut self,
        path: &Path,
        description: Option<String>,
        category: Option<CleanCategory>,
    ) -> Result<TrashEntry> {
        if !path.exists() {
            anyhow::bail!("Path does not exist: {}", path.display());
        }

        let is_directory = path.is_dir();
        let size = if is_directory {
            get_dir_size(path)?
        } else {
            fs::metadata(path)?.len()
        };

        // Generate unique trash path
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let unique_name = format!("{}_{}", file_name, timestamp);
        let trash_path = self.trash_dir.join(&unique_name);

        // Move to trash
        self.move_to_trash(path, &trash_path)?;

        // Create journal entry
        let description = description.unwrap_or_else(|| {
            if is_directory {
                format!("Directory: {}", path.display())
            } else {
                format!("File: {}", path.display())
            }
        });

        let entry = TrashEntry::new(
            path.to_path_buf(),
            trash_path,
            size,
            is_directory,
            description,
            category,
        );

        // Add to journal and save
        self.journal.add_entry(entry.clone());
        self.save_journal()?;

        info!("Moved to trash: {} -> {}", path.display(), entry.trash_path.display());

        // Auto-cleanup old entries if configured
        if let Some(days) = self.config.auto_empty_days {
            self.cleanup_old_entries(days)?;
        }

        Ok(entry)
    }

    /// Move file/directory to trash location
    fn move_to_trash(&self, source: &Path, dest: &Path) -> Result<()> {
        // Try rename first (fastest, works within same filesystem)
        match fs::rename(source, dest) {
            Ok(_) => return Ok(()),
            Err(e) => {
                debug!("Rename failed ({}), falling back to copy+delete", e);
            }
        }

        // Fall back to copy + delete (works across filesystems)
        if source.is_dir() {
            self.copy_dir_recursive(source, dest)?;
            fs::remove_dir_all(source)?;
        } else {
            fs::copy(source, dest)?;
            fs::remove_file(source)?;
        }

        Ok(())
    }

    /// Recursively copy a directory
    fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> Result<()> {
        fs::create_dir_all(dst)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                self.copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }

    /// Restore an item from trash by ID
    pub fn restore(&mut self, entry_id: &str) -> Result<PathBuf> {
        let entry = self
            .journal
            .find_entry(entry_id)
            .ok_or_else(|| anyhow::anyhow!("Trash entry not found: {}", entry_id))?
            .clone();

        self.restore_entry(&entry)
    }

    /// Restore an item from trash
    fn restore_entry(&mut self, entry: &TrashEntry) -> Result<PathBuf> {
        if !entry.trash_path.exists() {
            anyhow::bail!(
                "Trash file no longer exists: {}",
                entry.trash_path.display()
            );
        }

        // Check if original path is available
        let restore_path = if entry.original_path.exists() {
            // Generate alternative path
            let parent = entry.original_path.parent().unwrap_or(Path::new("/"));
            let file_name = entry
                .original_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("restored");
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            parent.join(format!("{}_{}", file_name, timestamp))
        } else {
            // Restore to original path
            entry.original_path.clone()
        };

        // Ensure parent directory exists
        if let Some(parent) = restore_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Move back
        fs::rename(&entry.trash_path, &restore_path)
            .with_context(|| format!("Failed to restore {} to {}", entry.trash_path.display(), restore_path.display()))?;

        // Remove from journal
        self.journal.remove_entry(&entry.id);
        self.save_journal()?;

        info!("Restored from trash: {} -> {}", entry.trash_path.display(), restore_path.display());

        Ok(restore_path)
    }

    /// Permanently delete an item from trash
    pub fn delete_permanently(&mut self, entry_id: &str) -> Result<u64> {
        let entry = self
            .journal
            .find_entry(entry_id)
            .ok_or_else(|| anyhow::anyhow!("Trash entry not found: {}", entry_id))?
            .clone();

        let size = entry.size;

        // Delete from filesystem
        if entry.trash_path.exists() {
            if entry.is_directory {
                fs::remove_dir_all(&entry.trash_path)?;
            } else {
                fs::remove_file(&entry.trash_path)?;
            }
        }

        // Remove from journal
        self.journal.remove_entry(entry_id);
        self.save_journal()?;

        info!("Permanently deleted from trash: {}", entry.trash_path.display());

        Ok(size)
    }

    /// Empty the entire trash
    pub fn empty(&mut self) -> Result<u64> {
        let total_size = self.journal.total_size();
        let entry_count = self.journal.list_entries().len();

        // Delete all files in trash directory
        if self.trash_dir.exists() {
            for entry in fs::read_dir(&self.trash_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    fs::remove_dir_all(&path)?;
                } else {
                    fs::remove_file(&path)?;
                }
            }
        }

        // Clear journal
        self.journal.clear();
        self.save_journal()?;

        info!("Emptied trash: {} items, {} bytes freed", entry_count, total_size);

        Ok(total_size)
    }

    /// List all items in trash
    pub fn list(&self) -> &[TrashEntry] {
        self.journal.list_entries()
    }

    /// Get total size of trash
    pub fn total_size(&self) -> u64 {
        self.journal.total_size()
    }

    /// Get number of items in trash
    pub fn count(&self) -> usize {
        self.journal.list_entries().len()
    }

    /// Cleanup entries older than specified days
    fn cleanup_old_entries(&mut self, days: u32) -> Result<u64> {
        let old_entries: Vec<_> = self
            .journal
            .entries_older_than(days)
            .iter()
            .map(|e| e.id.clone())
            .collect();

        let mut freed = 0u64;

        for id in old_entries {
            match self.delete_permanently(&id) {
                Ok(size) => freed += size,
                Err(e) => warn!("Failed to cleanup old entry {}: {}", id, e),
            }
        }

        if freed > 0 {
            info!("Auto-cleanup freed {} bytes", freed);
        }

        Ok(freed)
    }

    /// Find entry by ID or partial ID
    pub fn find_entry(&self, id_or_partial: &str) -> Option<&TrashEntry> {
        // Try exact match first
        if let Some(entry) = self.journal.find_entry(id_or_partial) {
            return Some(entry);
        }

        // Try partial match
        self.journal
            .list_entries()
            .iter()
            .find(|e| e.id.starts_with(id_or_partial))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_manager() -> (TrashManager, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let trash_path = dir.path().join("trash");

        let config = TrashConfig {
            use_system_trash: false,
            custom_trash_path: Some(trash_path),
            auto_empty_days: None,
            max_trash_size: None,
        };

        // Use isolated manager with empty journal to avoid conflicts between tests
        let manager = TrashManager::new_isolated(config).unwrap();
        (manager, dir)
    }

    #[test]
    fn test_trash_file() {
        let (mut manager, dir) = create_test_manager();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, b"test content").unwrap();

        let entry = manager.trash(&file_path).unwrap();

        assert!(!file_path.exists());
        assert!(entry.trash_path.exists());
        assert_eq!(entry.size, 12); // "test content".len()
        assert!(!entry.is_directory);
    }

    #[test]
    fn test_trash_directory() {
        let (mut manager, dir) = create_test_manager();
        let target_dir = dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();
        fs::write(target_dir.join("file.txt"), b"hello").unwrap();

        let entry = manager.trash(&target_dir).unwrap();

        assert!(!target_dir.exists());
        assert!(entry.trash_path.exists());
        assert!(entry.is_directory);
    }

    #[test]
    fn test_restore_file() {
        let (mut manager, dir) = create_test_manager();
        let file_path = dir.path().join("restore_test.txt");
        fs::write(&file_path, b"restore me").unwrap();

        let entry = manager.trash(&file_path).unwrap();
        assert!(!file_path.exists());

        let restored_path = manager.restore(&entry.id).unwrap();
        assert!(restored_path.exists());
        assert_eq!(fs::read_to_string(&restored_path).unwrap(), "restore me");
    }

    #[test]
    fn test_delete_permanently() {
        let (mut manager, dir) = create_test_manager();
        let file_path = dir.path().join("permanent.txt");
        fs::write(&file_path, b"gone forever").unwrap();

        let entry = manager.trash(&file_path).unwrap();
        let trash_path = entry.trash_path.clone();
        assert!(trash_path.exists());

        manager.delete_permanently(&entry.id).unwrap();
        assert!(!trash_path.exists());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_empty_trash() {
        let (mut manager, dir) = create_test_manager();

        // Add multiple files
        for i in 0..3 {
            let file_path = dir.path().join(format!("file{}.txt", i));
            fs::write(&file_path, b"content").unwrap();
            manager.trash(&file_path).unwrap();
        }

        assert_eq!(manager.count(), 3);

        manager.empty().unwrap();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_list_entries() {
        let (mut manager, dir) = create_test_manager();
        let file_path = dir.path().join("list_test.txt");
        fs::write(&file_path, b"test").unwrap();

        manager.trash(&file_path).unwrap();

        let entries = manager.list();
        assert_eq!(entries.len(), 1);
    }
}
