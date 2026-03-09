pub mod cache;
pub mod cleaner;
pub mod mapper;
pub mod platform;
pub mod scanner;
pub mod types;

pub use scanner::scan;
pub use types::{
    CleanCategory, CleanableItem, IgnoreList, RiskLevel, ScanConfig, ScanResults, ScanSpeed,
};
