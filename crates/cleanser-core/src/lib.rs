//! Cleanser Core Library
//!
//! This crate provides the core functionality for the cleanser disk cleanup tool.
//! It can be used as a library by CLI tools, GUI applications, or other Rust programs.
//!
//! # Example
//!
//! ```ignore
//! use cleanser_core::{scan, ScanConfig, ScanSpeed};
//!
//! let config = ScanConfig {
//!     speed: ScanSpeed::Normal,
//!     paths: vec![std::env::home_dir().unwrap()],
//!     ..Default::default()
//! };
//!
//! let results = scan(config)?;
//! println!("Found {} cleanable items", results.items.len());
//! ```
//!
//! # Progress Callbacks
//!
//! For progress updates, implement the `ProgressCallback` trait:
//!
//! ```ignore
//! use cleanser_core::{ProgressCallback, ScanProgress, CleanProgress, scan_with_progress};
//!
//! struct MyProgress;
//!
//! impl ProgressCallback for MyProgress {
//!     fn on_scan_progress(&self, progress: ScanProgress) {
//!         println!("{}", progress.message);
//!     }
//!     fn on_clean_progress(&self, progress: CleanProgress) {
//!         println!("{}/{}", progress.current, progress.total);
//!     }
//! }
//! ```

pub mod cache;
pub mod cleaner;
pub mod mapper;
pub mod platform;
pub mod progress;
pub mod scanner;
pub mod types;
pub mod utils;
pub mod version;

// Re-export main types
pub use types::{
    AgeCriteria, CleanCategory, CleanResult, CleanableItem, IgnoreList, RiskLevel, ScanConfig,
    ScanResults, ScanSpeed, SizeRange, WhitelistConfig,
};

// Re-export progress types
pub use progress::{
    CleanPhase, CleanProgress, NoOpProgress, ProgressCallback, ScanPhase, ScanProgress,
};

// Re-export main functions
pub use cleaner::{
    clean, clean_with_progress, delete_items, delete_items_with_progress, filter_by_risk,
};
pub use scanner::{scan, scan_with_progress};

// Re-export cache functions
pub use cache::{
    clear_cache, get_cache_age, load_scan_results, save_scan_results, update_cache_after_deletion,
};

// Re-export platform utilities
pub use platform::{home_dir, home_dir_or_err, Platform};

// Re-export mapper types
pub use mapper::crawler::CrawlerProgress;
pub use mapper::{
    DirectoryCategory, FileSystemCrawler, FileSystemMap, MappedDirectory, PathClassifier,
};

// Re-export duration parser
pub use types::parse_duration;

// Re-export utility functions
pub use utils::get_dir_size;

// Re-export version utilities
pub use version::{current_version, VersionInfo};
#[cfg(feature = "update-check")]
pub use version::{check_for_updates, check_for_updates_background, check_for_updates_sync};
