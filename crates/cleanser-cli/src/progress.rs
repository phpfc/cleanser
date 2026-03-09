//! CLI progress callbacks using indicatif.

use cleanser_core::{CleanProgress, ProgressCallback, ScanProgress};
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Mutex;

/// CLI progress callback that uses indicatif for terminal output
pub struct CliProgress {
    pb: Mutex<ProgressBar>,
}

impl CliProgress {
    pub fn new_spinner() -> Self {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .expect("Failed to create progress bar template"),
        );
        Self { pb: Mutex::new(pb) }
    }

    pub fn finish(&self) {
        if let Ok(pb) = self.pb.lock() {
            pb.finish_and_clear();
        }
    }
}

impl ProgressCallback for CliProgress {
    fn on_scan_progress(&self, progress: ScanProgress) {
        if let Ok(pb) = self.pb.lock() {
            pb.set_message(progress.message);
        }
    }

    fn on_clean_progress(&self, progress: CleanProgress) {
        if let Ok(pb) = self.pb.lock() {
            pb.set_position(progress.current);
            pb.set_length(progress.total);
            pb.set_message(progress.message);
        }
    }
}

impl Drop for CliProgress {
    fn drop(&mut self) {
        self.finish();
    }
}
