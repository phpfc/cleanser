//! Progress callback abstraction for scan and clean operations.
//!
//! This module provides a trait-based callback system that allows consumers
//! (CLI, GUI, etc.) to receive progress updates without coupling the core
//! library to any specific UI framework.

use serde::{Deserialize, Serialize};

/// Phase of a scan operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanPhase {
    /// Loading the filesystem map
    LoadingMap,
    /// Updating a stale filesystem map
    UpdatingMap,
    /// Actively scanning directories
    Scanning,
    /// Finding duplicate files
    FindingDuplicates,
    /// Scan complete
    Complete,
}

/// Phase of a clean operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CleanPhase {
    /// Loading cached scan results
    Loading,
    /// Running a fresh scan
    Scanning,
    /// Actively cleaning files
    Cleaning,
    /// Clean complete
    Complete,
}

/// Progress update for scan operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub phase: ScanPhase,
    pub message: String,
    pub current: Option<u64>,
    pub total: Option<u64>,
}

/// Progress update for clean operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanProgress {
    pub phase: CleanPhase,
    pub message: String,
    pub current_item: Option<String>,
    pub current: u64,
    pub total: u64,
    pub cleaned_size: u64,
}

/// Trait for receiving progress updates during operations.
///
/// Implement this trait to receive progress notifications from scan and clean operations.
/// The callbacks are called synchronously, so implementations should be fast to avoid
/// blocking the operation.
///
/// # Example
///
/// ```ignore
/// use cleanser_core::{ProgressCallback, ScanProgress, CleanProgress};
///
/// struct MyProgress;
///
/// impl ProgressCallback for MyProgress {
///     fn on_scan_progress(&self, progress: ScanProgress) {
///         println!("{}: {}", progress.phase, progress.message);
///     }
///
///     fn on_clean_progress(&self, progress: CleanProgress) {
///         println!("Cleaning {}/{}", progress.current, progress.total);
///     }
/// }
/// ```
pub trait ProgressCallback: Send + Sync {
    /// Called when scan progress updates
    fn on_scan_progress(&self, progress: ScanProgress);

    /// Called when clean progress updates
    fn on_clean_progress(&self, progress: CleanProgress);
}

/// A no-op progress callback that ignores all updates.
///
/// Use this when you don't need progress updates.
pub struct NoOpProgress;

impl ProgressCallback for NoOpProgress {
    fn on_scan_progress(&self, _progress: ScanProgress) {}
    fn on_clean_progress(&self, _progress: CleanProgress) {}
}

impl Default for NoOpProgress {
    fn default() -> Self {
        Self
    }
}
