//! Trash journal for tracking deleted items.
//!
//! This module provides persistence for trash operations, allowing
//! items to be tracked and restored from the trash.

use crate::platform::Platform;
use crate::types::CleanCategory;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Current journal schema version
const JOURNAL_VERSION: u32 = 1;

/// Entry in the trash journal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashEntry {
    /// Unique identifier for this entry
    pub id: String,
    /// Original path of the file/directory
    pub original_path: PathBuf,
    /// Path in trash location
    pub trash_path: PathBuf,
    /// Size in bytes
    pub size: u64,
    /// Whether this is a directory
    pub is_directory: bool,
    /// When the item was deleted
    pub deleted_at: DateTime<Utc>,
    /// Description of the item
    pub description: String,
    /// Category from the original CleanableItem (if applicable)
    pub category: Option<CleanCategory>,
}

impl TrashEntry {
    /// Create a new trash entry
    pub fn new(
        original_path: PathBuf,
        trash_path: PathBuf,
        size: u64,
        is_directory: bool,
        description: String,
        category: Option<CleanCategory>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            original_path,
            trash_path,
            size,
            is_directory,
            deleted_at: Utc::now(),
            description,
            category,
        }
    }

    /// Get a human-readable age string
    pub fn age_string(&self) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.deleted_at);

        if duration.num_days() > 0 {
            format!("{} days ago", duration.num_days())
        } else if duration.num_hours() > 0 {
            format!("{} hours ago", duration.num_hours())
        } else if duration.num_minutes() > 0 {
            format!("{} minutes ago", duration.num_minutes())
        } else {
            "just now".to_string()
        }
    }
}

/// Trash journal for persistence
#[derive(Debug, Serialize, Deserialize)]
pub struct TrashJournal {
    /// Schema version
    pub version: u32,
    /// All trash entries
    pub entries: Vec<TrashEntry>,
    /// Last time the trash was cleaned up
    pub last_cleanup: Option<DateTime<Utc>>,
}

impl Default for TrashJournal {
    fn default() -> Self {
        Self {
            version: JOURNAL_VERSION,
            entries: Vec::new(),
            last_cleanup: None,
        }
    }
}

impl TrashJournal {
    /// Get the journal file path
    pub fn get_journal_path() -> Result<PathBuf> {
        let config_dir = match Platform::current() {
            Platform::MacOS => dirs::data_local_dir()
                .map(|p| p.join("cleanser"))
                .or_else(|| dirs::home_dir().map(|p| p.join(".cleanser"))),
            Platform::Linux => dirs::config_dir()
                .map(|p| p.join("cleanser"))
                .or_else(|| dirs::home_dir().map(|p| p.join(".config/cleanser"))),
            Platform::Windows => dirs::data_local_dir()
                .map(|p| p.join("cleanser"))
                .or_else(|| dirs::home_dir().map(|p| p.join(".cleanser"))),
        };

        let config_dir =
            config_dir.ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

        Ok(config_dir.join("trash-journal.json"))
    }

    /// Get the lock file path
    fn get_lock_path() -> Result<PathBuf> {
        let journal_path = Self::get_journal_path()?;
        Ok(journal_path.with_extension("lock"))
    }

    /// Load the journal from disk with file locking
    pub fn load() -> Result<Self> {
        let journal_path = Self::get_journal_path()?;

        if !journal_path.exists() {
            return Ok(Self::default());
        }

        // Acquire shared lock for reading
        let lock_path = Self::get_lock_path()?;
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let lock_file = File::create(&lock_path)?;
        lock_file
            .lock_shared()
            .context("Failed to acquire shared lock on journal")?;

        let content = fs::read_to_string(&journal_path)
            .with_context(|| format!("Failed to read journal from {}", journal_path.display()))?;

        let journal: Self = serde_json::from_str(&content)
            .with_context(|| "Failed to parse journal JSON")?;

        lock_file.unlock()?;

        // Check version and migrate if needed
        if journal.version != JOURNAL_VERSION {
            warn!(
                "Journal version mismatch: expected {}, got {}",
                JOURNAL_VERSION, journal.version
            );
        }

        debug!("Loaded trash journal with {} entries", journal.entries.len());
        Ok(journal)
    }

    /// Save the journal to disk with file locking
    pub fn save(&self) -> Result<()> {
        let journal_path = Self::get_journal_path()?;

        if let Some(parent) = journal_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Acquire exclusive lock for writing
        let lock_path = Self::get_lock_path()?;
        let lock_file = File::create(&lock_path)?;
        lock_file
            .lock_exclusive()
            .context("Failed to acquire exclusive lock on journal")?;

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize journal")?;

        fs::write(&journal_path, content)
            .with_context(|| format!("Failed to write journal to {}", journal_path.display()))?;

        lock_file.unlock()?;

        debug!("Saved trash journal with {} entries", self.entries.len());
        Ok(())
    }

    /// Add an entry to the journal
    pub fn add_entry(&mut self, entry: TrashEntry) {
        self.entries.push(entry);
    }

    /// Remove an entry by ID
    pub fn remove_entry(&mut self, id: &str) -> Option<TrashEntry> {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            Some(self.entries.remove(pos))
        } else {
            None
        }
    }

    /// Find an entry by ID
    pub fn find_entry(&self, id: &str) -> Option<&TrashEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Find an entry by original path
    pub fn find_by_original_path(&self, path: &Path) -> Option<&TrashEntry> {
        self.entries.iter().find(|e| e.original_path == path)
    }

    /// Get all entries
    pub fn list_entries(&self) -> &[TrashEntry] {
        &self.entries
    }

    /// Get total size of all entries
    pub fn total_size(&self) -> u64 {
        self.entries.iter().map(|e| e.size).sum()
    }

    /// Get entries older than a certain number of days
    pub fn entries_older_than(&self, days: u32) -> Vec<&TrashEntry> {
        let cutoff = Utc::now() - chrono::Duration::days(days as i64);
        self.entries
            .iter()
            .filter(|e| e.deleted_at < cutoff)
            .collect()
    }

    /// Clear all entries (for when trash is emptied)
    pub fn clear(&mut self) {
        self.entries.clear();
        self.last_cleanup = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_trash_entry_creation() {
        let entry = TrashEntry::new(
            PathBuf::from("/original/path"),
            PathBuf::from("/trash/path"),
            1024,
            false,
            "Test file".to_string(),
            None,
        );

        assert!(!entry.id.is_empty());
        assert_eq!(entry.size, 1024);
        assert!(!entry.is_directory);
    }

    #[test]
    fn test_journal_default() {
        let journal = TrashJournal::default();
        assert_eq!(journal.version, JOURNAL_VERSION);
        assert!(journal.entries.is_empty());
        assert!(journal.last_cleanup.is_none());
    }

    #[test]
    fn test_journal_add_remove_entry() {
        let mut journal = TrashJournal::default();

        let entry = TrashEntry::new(
            PathBuf::from("/test"),
            PathBuf::from("/trash/test"),
            100,
            false,
            "Test".to_string(),
            None,
        );
        let id = entry.id.clone();

        journal.add_entry(entry);
        assert_eq!(journal.entries.len(), 1);

        let removed = journal.remove_entry(&id);
        assert!(removed.is_some());
        assert_eq!(journal.entries.len(), 0);
    }

    #[test]
    fn test_journal_find_entry() {
        let mut journal = TrashJournal::default();

        let entry = TrashEntry::new(
            PathBuf::from("/test/file.txt"),
            PathBuf::from("/trash/file.txt"),
            100,
            false,
            "Test".to_string(),
            None,
        );
        let id = entry.id.clone();
        journal.add_entry(entry);

        assert!(journal.find_entry(&id).is_some());
        assert!(journal.find_entry("nonexistent").is_none());
        assert!(journal
            .find_by_original_path(Path::new("/test/file.txt"))
            .is_some());
    }

    #[test]
    fn test_journal_total_size() {
        let mut journal = TrashJournal::default();

        journal.add_entry(TrashEntry::new(
            PathBuf::from("/a"),
            PathBuf::from("/t/a"),
            100,
            false,
            "A".to_string(),
            None,
        ));
        journal.add_entry(TrashEntry::new(
            PathBuf::from("/b"),
            PathBuf::from("/t/b"),
            200,
            false,
            "B".to_string(),
            None,
        ));

        assert_eq!(journal.total_size(), 300);
    }
}
