//! Progress callback implementation for Tauri events.

use cleanser_core::{CleanProgress, ProgressCallback, ScanProgress};
use serde::Serialize;
use std::sync::Arc;
use tauri::{Emitter, Window};

/// Progress callback that emits Tauri events to the frontend.
pub struct TauriProgress {
    window: Arc<Window>,
}

impl TauriProgress {
    pub fn new(window: Window) -> Self {
        Self {
            window: Arc::new(window),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct ScanProgressEvent {
    pub phase: String,
    pub message: String,
    pub current: Option<u64>,
    pub total: Option<u64>,
}

#[derive(Clone, Serialize)]
pub struct CleanProgressEvent {
    pub phase: String,
    pub message: String,
    pub current_item: Option<String>,
    pub current: u64,
    pub total: u64,
    pub cleaned_size: u64,
}

impl ProgressCallback for TauriProgress {
    fn on_scan_progress(&self, progress: ScanProgress) {
        let event = ScanProgressEvent {
            phase: format!("{:?}", progress.phase),
            message: progress.message,
            current: progress.current,
            total: progress.total,
        };
        let _ = self.window.emit("scan:progress", event);
    }

    fn on_clean_progress(&self, progress: CleanProgress) {
        let event = CleanProgressEvent {
            phase: format!("{:?}", progress.phase),
            message: progress.message,
            current_item: progress.current_item,
            current: progress.current,
            total: progress.total,
            cleaned_size: progress.cleaned_size,
        };
        let _ = self.window.emit("clean:progress", event);
    }
}
