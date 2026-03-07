use clap::ValueEnum;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Copy, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanSpeed {
    /// Quick scan - only common cache locations
    Quick,
    /// Normal scan - balanced speed and coverage
    Normal,
    /// Thorough scan - deep scan of all locations
    Thorough,
}

impl fmt::Display for ScanSpeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScanSpeed::Quick => write!(f, "quick"),
            ScanSpeed::Normal => write!(f, "normal"),
            ScanSpeed::Thorough => write!(f, "thorough"),
        }
    }
}

#[derive(
    Debug, Clone, Copy, ValueEnum, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// Safe to delete - caches, logs, temp files
    Safe,
    /// Moderate risk - development artifacts, can be regenerated
    Moderate,
    /// Higher risk - large files, duplicates, requires review
    Risky,
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RiskLevel::Safe => write!(f, "safe"),
            RiskLevel::Moderate => write!(f, "moderate"),
            RiskLevel::Risky => write!(f, "risky"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanableItem {
    #[serde(serialize_with = "serialize_pathbuf", deserialize_with = "deserialize_pathbuf")]
    pub path: PathBuf,
    pub size: u64,
    pub category: CleanCategory,
    pub risk_level: RiskLevel,
    pub description: String,
}

fn serialize_pathbuf<S>(path: &Path, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&path.to_string_lossy())
}

fn deserialize_pathbuf<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(PathBuf::from(s))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CleanCategory {
    SystemCache,
    BrowserCache,
    AppCache,
    SystemLogs,
    AppLogs,
    TempFiles,
    NodeModules,
    BuildArtifacts,
    PipCache,
    BrewCache,
    CargoCache,
    LargeFiles,
    DuplicateFiles,
}

impl fmt::Display for CleanCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CleanCategory::SystemCache => write!(f, "System Cache"),
            CleanCategory::BrowserCache => write!(f, "Browser Cache"),
            CleanCategory::AppCache => write!(f, "Application Cache"),
            CleanCategory::SystemLogs => write!(f, "System Logs"),
            CleanCategory::AppLogs => write!(f, "Application Logs"),
            CleanCategory::TempFiles => write!(f, "Temporary Files"),
            CleanCategory::NodeModules => write!(f, "Node Modules"),
            CleanCategory::BuildArtifacts => write!(f, "Build Artifacts"),
            CleanCategory::PipCache => write!(f, "Pip Cache"),
            CleanCategory::BrewCache => write!(f, "Homebrew Cache"),
            CleanCategory::CargoCache => write!(f, "Cargo Cache"),
            CleanCategory::LargeFiles => write!(f, "Large Files"),
            CleanCategory::DuplicateFiles => write!(f, "Duplicate Files"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResults {
    pub items: Vec<CleanableItem>,
    pub total_size: u64,
    pub scan_speed: ScanSpeed,
    pub excluded_dirs_count: usize,
    pub filtered_by_size_count: usize,
    pub filtered_by_age_count: usize,
}

#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub speed: ScanSpeed,
    pub paths: Vec<PathBuf>,
    pub min_file_size_mb: u64,
    pub max_depth: Option<usize>,
    pub find_duplicates: bool,
    pub ignore_patterns: IgnoreList,
    pub size_range: Option<SizeRange>,
    pub age_criteria: Option<AgeCriteria>,
    #[allow(dead_code)]
    pub interactive_mode: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileHash {
    pub hash: String,
    pub size: u64,
}

// ===== Ignore Pattern System =====

#[derive(Debug, Clone)]
pub struct IgnorePattern {
    pub path: PathBuf,
    #[allow(dead_code)]
    pub is_absolute: bool,
}

#[derive(Debug, Clone, Default)]
pub struct IgnoreList {
    pub patterns: Vec<IgnorePattern>,
}

impl IgnoreList {
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    pub fn add_pattern(&mut self, path: &str) -> anyhow::Result<()> {
        let expanded_path = if path.starts_with('~') {
            let home = std::env::var("HOME")?;
            PathBuf::from(path.replacen('~', &home, 1))
        } else {
            PathBuf::from(path)
        };

        let is_absolute = expanded_path.is_absolute();
        self.patterns.push(IgnorePattern {
            path: expanded_path,
            is_absolute,
        });
        Ok(())
    }

    pub fn should_ignore(&self, path: &Path) -> bool {
        for pattern in &self.patterns {
            if path.starts_with(&pattern.path) {
                return true;
            }
        }
        false
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.patterns.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

// ===== Size Range Filtering =====

#[derive(Debug, Clone)]
pub struct SizeRange {
    pub min: Option<u64>,
    pub max: Option<u64>,
}

impl SizeRange {
    pub fn parse(input: &str) -> anyhow::Result<Self> {
        let parts: Vec<&str> = input.split('-').collect();
        
        if parts.len() != 2 {
            anyhow::bail!("Invalid size range format. Expected format: '100MB-500MB', '100MB-', or '-500MB'");
        }

        let min = if parts[0].is_empty() {
            None
        } else {
            Some(parse_size_with_unit(parts[0])?)
        };

        let max = if parts[1].is_empty() {
            None
        } else {
            Some(parse_size_with_unit(parts[1])?)
        };

        // Validate min <= max
        if let (Some(min_val), Some(max_val)) = (min, max) {
            if min_val > max_val {
                anyhow::bail!("Minimum size ({}) cannot be greater than maximum size ({})", 
                    humansize::format_size(min_val, humansize::BINARY),
                    humansize::format_size(max_val, humansize::BINARY));
            }
        }

        Ok(Self { min, max })
    }

    pub fn contains(&self, size: u64) -> bool {
        let min_ok = self.min.map_or(true, |min| size >= min);
        let max_ok = self.max.map_or(true, |max| size <= max);
        min_ok && max_ok
    }
}

fn parse_size_with_unit(input: &str) -> anyhow::Result<u64> {
    let input = input.trim().to_uppercase();
    
    let (num_str, unit) = if input.ends_with("TB") {
        (&input[..input.len() - 2], 1024u64.pow(4))
    } else if input.ends_with("GB") {
        (&input[..input.len() - 2], 1024u64.pow(3))
    } else if input.ends_with("MB") {
        (&input[..input.len() - 2], 1024u64.pow(2))
    } else if input.ends_with("KB") {
        (&input[..input.len() - 2], 1024u64)
    } else if input.ends_with('B') {
        (&input[..input.len() - 1], 1u64)
    } else {
        // Assume MB if no unit specified
        (input.as_str(), 1024u64.pow(2))
    };

    let num: f64 = num_str.trim().parse()
        .map_err(|_| anyhow::anyhow!("Invalid number in size: {}", input))?;
    
    Ok((num * unit as f64) as u64)
}

// ===== Age-Based Filtering =====

#[derive(Debug, Clone)]
pub struct AgeCriteria {
    pub older_than: Option<Duration>,
    pub newer_than: Option<Duration>,
}

impl AgeCriteria {
    pub fn new() -> Self {
        Self {
            older_than: None,
            newer_than: None,
        }
    }

    pub fn set_older_than(&mut self, duration: Duration) {
        self.older_than = Some(duration);
    }

    pub fn set_newer_than(&mut self, duration: Duration) {
        self.newer_than = Some(duration);
    }

    pub fn matches(&self, modified_time: SystemTime) -> bool {
        let now = SystemTime::now();
        let age = now.duration_since(modified_time).unwrap_or(Duration::from_secs(0));

        let older_ok = self.older_than.map_or(true, |threshold| age >= threshold);
        let newer_ok = self.newer_than.map_or(true, |threshold| age <= threshold);

        older_ok && newer_ok
    }
}

pub fn parse_duration(input: &str) -> anyhow::Result<Duration> {
    let input = input.trim().to_lowercase();
    
    let (num_str, multiplier) = if input.ends_with('y') {
        (&input[..input.len() - 1], 365 * 24 * 60 * 60)
    } else if input.ends_with('m') {
        (&input[..input.len() - 1], 30 * 24 * 60 * 60)
    } else if input.ends_with('w') {
        (&input[..input.len() - 1], 7 * 24 * 60 * 60)
    } else if input.ends_with('d') {
        (&input[..input.len() - 1], 24 * 60 * 60)
    } else {
        anyhow::bail!("Invalid duration format. Expected format: '90d', '2w', '6m', or '1y'");
    };

    let num: u64 = num_str.trim().parse()
        .map_err(|_| anyhow::anyhow!("Invalid number in duration: {}", input))?;
    
    Ok(Duration::from_secs(num * multiplier))
}

// ===== Whitelist Configuration =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelistConfig {
    pub version: String,
    pub whitelist: HashSet<PathBuf>,
}

impl WhitelistConfig {
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            whitelist: HashSet::new(),
        }
    }

    pub fn get_config_path() -> anyhow::Result<PathBuf> {
        let home = std::env::var("HOME")?;
        Ok(PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("cleanser")
            .join("whitelist.json"))
    }

    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::get_config_path()?;

        if !config_path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(&config_path)?;
        let mut config: WhitelistConfig = serde_json::from_str(&content)?;

        // Migrate relative paths to absolute paths
        let home = std::env::var("HOME")?;
        let home_path = PathBuf::from(&home);

        let mut normalized_whitelist = HashSet::new();
        let mut needs_save = false;

        for path in config.whitelist.iter() {
            if path.is_relative() {
                // Convert relative path to absolute by joining with home directory
                let absolute_path = home_path.join(path);
                normalized_whitelist.insert(absolute_path);
                needs_save = true;
            } else {
                normalized_whitelist.insert(path.clone());
            }
        }

        config.whitelist = normalized_whitelist;

        // Save the normalized config if any paths were converted
        if needs_save {
            config.save()?;
        }

        Ok(config)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::get_config_path()?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    pub fn add_path(&mut self, path: PathBuf) -> anyhow::Result<()> {
        // Convert to absolute path if relative
        let absolute_path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()?.join(&path)
        };

        // Canonicalize to resolve symlinks and normalize the path
        let canonical_path = match absolute_path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // If canonicalize fails (e.g., path doesn't exist), use the absolute path as-is
                absolute_path
            }
        };

        self.whitelist.insert(canonical_path);
        self.save()?;
        Ok(())
    }

    pub fn remove_path(&mut self, path: &Path) -> anyhow::Result<bool> {
        // Try to remove by exact match first
        let mut removed = self.whitelist.remove(path);

        // If not found and path is relative, try converting to absolute
        if !removed && path.is_relative() {
            let absolute_path = std::env::current_dir()?.join(path);
            if let Ok(canonical_path) = absolute_path.canonicalize() {
                removed = self.whitelist.remove(&canonical_path);
            }
            // Also try with home directory
            if !removed {
                if let Ok(home) = std::env::var("HOME") {
                    let home_path = PathBuf::from(home).join(path);
                    removed = self.whitelist.remove(&home_path);
                }
            }
        }

        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.whitelist.iter().any(|p| path.starts_with(p))
    }

    pub fn list_paths(&self) -> Vec<&PathBuf> {
        self.whitelist.iter().collect()
    }
}

impl Default for WhitelistConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ===== TUI Data Structures =====

#[derive(Debug, Clone)]
pub struct FileItem {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub risk_level: RiskLevel,
    pub selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    SizeDesc,      // Heaviest first (default)
    SizeAsc,       // Lightest first
    DateOldest,    // Oldest first
    DateNewest,    // Newest first
    PathAsc,       // Path A-Z
    PathDesc,      // Path Z-A
}

impl std::fmt::Display for SortMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortMode::SizeDesc => write!(f, "Size ↓ (Heaviest first)"),
            SortMode::SizeAsc => write!(f, "Size ↑ (Lightest first)"),
            SortMode::DateOldest => write!(f, "Date ↑ (Oldest first)"),
            SortMode::DateNewest => write!(f, "Date ↓ (Newest first)"),
            SortMode::PathAsc => write!(f, "Path ↑ (A-Z)"),
            SortMode::PathDesc => write!(f, "Path ↓ (Z-A)"),
        }
    }
}

pub struct TuiState {
    pub items: Vec<FileItem>,
    pub cursor_position: usize,
    pub scroll_offset: usize,
    pub selected_count: usize,
    pub total_selected_size: u64,
    pub sort_mode: SortMode,
}

impl TuiState {
    pub fn new(mut items: Vec<FileItem>) -> Self {
        // Sort by size descending by default
        items.sort_by(|a, b| b.size.cmp(&a.size));

        Self {
            items,
            cursor_position: 0,
            scroll_offset: 0,
            selected_count: 0,
            total_selected_size: 0,
            sort_mode: SortMode::SizeDesc,
        }
    }

    pub fn move_cursor_up(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_down(&mut self) {
        if self.cursor_position < self.items.len().saturating_sub(1) {
            self.cursor_position += 1;
        }
    }

    pub fn toggle_selection(&mut self) {
        if let Some(item) = self.items.get_mut(self.cursor_position) {
            item.selected = !item.selected;
            if item.selected {
                self.selected_count += 1;
                self.total_selected_size += item.size;
            } else {
                self.selected_count = self.selected_count.saturating_sub(1);
                self.total_selected_size = self.total_selected_size.saturating_sub(item.size);
            }
        }
    }

    pub fn get_selected_items(&self) -> Vec<&FileItem> {
        self.items.iter().filter(|item| item.selected).collect()
    }

    pub fn set_sort_mode(&mut self, mode: SortMode) {
        self.sort_mode = mode;
        self.sort_items();
    }

    pub fn cycle_sort_mode(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::SizeDesc => SortMode::SizeAsc,
            SortMode::SizeAsc => SortMode::DateOldest,
            SortMode::DateOldest => SortMode::DateNewest,
            SortMode::DateNewest => SortMode::PathAsc,
            SortMode::PathAsc => SortMode::PathDesc,
            SortMode::PathDesc => SortMode::SizeDesc,
        };
        self.sort_items();
    }

    fn sort_items(&mut self) {
        match self.sort_mode {
            SortMode::SizeDesc => {
                self.items.sort_by(|a, b| b.size.cmp(&a.size));
            }
            SortMode::SizeAsc => {
                self.items.sort_by(|a, b| a.size.cmp(&b.size));
            }
            SortMode::DateOldest => {
                self.items.sort_by(|a, b| a.modified.cmp(&b.modified));
            }
            SortMode::DateNewest => {
                self.items.sort_by(|a, b| b.modified.cmp(&a.modified));
            }
            SortMode::PathAsc => {
                self.items.sort_by(|a, b| a.path.cmp(&b.path));
            }
            SortMode::PathDesc => {
                self.items.sort_by(|a, b| b.path.cmp(&a.path));
            }
        }

        // Reset cursor to top after sorting
        self.cursor_position = 0;
        self.scroll_offset = 0;
    }
}

// ===== Interactive Prompt =====

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct InteractivePrompt {
    pub current_index: usize,
    pub total_files: usize,
    pub deleted_count: usize,
    pub skipped_count: usize,
}

impl InteractivePrompt {
    pub fn new(total_files: usize) -> Self {
        Self {
            current_index: 0,
            total_files,
            deleted_count: 0,
            skipped_count: 0,
        }
    }

    pub fn display_summary(&self) {
        use colored::Colorize;

        println!("\n{}", "=== Interactive Deletion Summary ===".cyan().bold());
        println!("Total files processed: {}", self.current_index);
        println!("{}: {}", "Files deleted".green(), self.deleted_count);
        println!("{}: {}", "Files skipped".yellow(), self.skipped_count);
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserChoice {
    Delete,
    Skip,
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Safe < RiskLevel::Moderate);
        assert!(RiskLevel::Moderate < RiskLevel::Risky);
        assert!(RiskLevel::Safe < RiskLevel::Risky);
    }

    #[test]
    fn test_scan_speed_display() {
        assert_eq!(format!("{}", ScanSpeed::Quick), "quick");
        assert_eq!(format!("{}", ScanSpeed::Normal), "normal");
        assert_eq!(format!("{}", ScanSpeed::Thorough), "thorough");
    }

    #[test]
    fn test_risk_level_display() {
        assert_eq!(format!("{}", RiskLevel::Safe), "safe");
        assert_eq!(format!("{}", RiskLevel::Moderate), "moderate");
        assert_eq!(format!("{}", RiskLevel::Risky), "risky");
    }

    #[test]
    fn test_clean_category_display() {
        assert_eq!(format!("{}", CleanCategory::SystemCache), "System Cache");
        assert_eq!(format!("{}", CleanCategory::NodeModules), "Node Modules");
        assert_eq!(format!("{}", CleanCategory::LargeFiles), "Large Files");
    }

    #[test]
    fn test_pathbuf_serialization() {
        let item = CleanableItem {
            path: PathBuf::from("/test/path"),
            size: 1024,
            category: CleanCategory::SystemCache,
            risk_level: RiskLevel::Safe,
            description: "Test item".to_string(),
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("/test/path"));
        assert!(json.contains("1024"));

        let deserialized: CleanableItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, PathBuf::from("/test/path"));
        assert_eq!(deserialized.size, 1024);
    }

    #[test]
    fn test_file_hash_equality() {
        let hash1 = FileHash {
            hash: "abc123".to_string(),
            size: 1024,
        };
        let hash2 = FileHash {
            hash: "abc123".to_string(),
            size: 1024,
        };
        let hash3 = FileHash {
            hash: "def456".to_string(),
            size: 1024,
        };

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_ignore_list_add_pattern() {
        let mut ignore_list = IgnoreList::new();
        assert!(ignore_list.add_pattern("/test/path").is_ok());
        assert_eq!(ignore_list.len(), 1);
    }

    #[test]
    fn test_ignore_list_should_ignore() {
        let mut ignore_list = IgnoreList::new();
        ignore_list.add_pattern("/test/ignore").unwrap();
        
        assert!(ignore_list.should_ignore(Path::new("/test/ignore")));
        assert!(ignore_list.should_ignore(Path::new("/test/ignore/subdir")));
        assert!(!ignore_list.should_ignore(Path::new("/test/other")));
    }

    #[test]
    fn test_size_range_parse() {
        let range = SizeRange::parse("100MB-500MB").unwrap();
        assert!(range.min.is_some());
        assert!(range.max.is_some());

        let range = SizeRange::parse("100MB-").unwrap();
        assert!(range.min.is_some());
        assert!(range.max.is_none());

        let range = SizeRange::parse("-500MB").unwrap();
        assert!(range.min.is_none());
        assert!(range.max.is_some());
    }

    #[test]
    fn test_size_range_contains() {
        let range = SizeRange::parse("100MB-500MB").unwrap();
        assert!(!range.contains(50 * 1024 * 1024));
        assert!(range.contains(200 * 1024 * 1024));
        assert!(!range.contains(600 * 1024 * 1024));
    }

    #[test]
    fn test_parse_duration() {
        let duration = parse_duration("90d").unwrap();
        assert_eq!(duration.as_secs(), 90 * 24 * 60 * 60);

        let duration = parse_duration("2w").unwrap();
        assert_eq!(duration.as_secs(), 2 * 7 * 24 * 60 * 60);

        let duration = parse_duration("6m").unwrap();
        assert_eq!(duration.as_secs(), 6 * 30 * 24 * 60 * 60);

        let duration = parse_duration("1y").unwrap();
        assert_eq!(duration.as_secs(), 365 * 24 * 60 * 60);
    }

    #[test]
    fn test_age_criteria_matches() {
        let mut criteria = AgeCriteria::new();
        criteria.set_older_than(Duration::from_secs(60));

        let old_time = SystemTime::now() - Duration::from_secs(120);
        let recent_time = SystemTime::now() - Duration::from_secs(30);

        assert!(criteria.matches(old_time));
        assert!(!criteria.matches(recent_time));
    }

    #[test]
    fn test_tui_state_cursor_movement() {
        let items = vec![
            FileItem {
                path: PathBuf::from("/test1"),
                size: 100,
                modified: SystemTime::now(),
                risk_level: RiskLevel::Safe,
                selected: false,
            },
            FileItem {
                path: PathBuf::from("/test2"),
                size: 200,
                modified: SystemTime::now(),
                risk_level: RiskLevel::Safe,
                selected: false,
            },
        ];

        let mut state = TuiState::new(items);
        assert_eq!(state.cursor_position, 0);

        state.move_cursor_down();
        assert_eq!(state.cursor_position, 1);

        state.move_cursor_down();
        assert_eq!(state.cursor_position, 1); // Should not go beyond bounds

        state.move_cursor_up();
        assert_eq!(state.cursor_position, 0);

        state.move_cursor_up();
        assert_eq!(state.cursor_position, 0); // Should not go below 0
    }

    #[test]
    fn test_tui_state_toggle_selection() {
        let items = vec![
            FileItem {
                path: PathBuf::from("/test1"),
                size: 100,
                modified: SystemTime::now(),
                risk_level: RiskLevel::Safe,
                selected: false,
            },
        ];

        let mut state = TuiState::new(items);
        assert_eq!(state.selected_count, 0);
        assert_eq!(state.total_selected_size, 0);

        state.toggle_selection();
        assert_eq!(state.selected_count, 1);
        assert_eq!(state.total_selected_size, 100);

        state.toggle_selection();
        assert_eq!(state.selected_count, 0);
        assert_eq!(state.total_selected_size, 0);
    }
}
