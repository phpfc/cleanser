use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;

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
    pub path: String,
    pub size: u64,
    pub category: CleanCategory,
    pub risk_level: RiskLevel,
    pub description: String,
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
    pub paths: Vec<String>,
    pub min_file_size_mb: u64,
    pub max_depth: Option<usize>,
    pub find_duplicates: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileHash {
    pub hash: String,
    pub size: u64,
}
