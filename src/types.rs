use clap::ValueEnum;
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;
use std::path::{Path, PathBuf};

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
}

#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub speed: ScanSpeed,
    pub paths: Vec<PathBuf>,
    pub min_file_size_mb: u64,
    pub max_depth: Option<usize>,
    pub find_duplicates: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileHash {
    pub hash: String,
    pub size: u64,
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
}
