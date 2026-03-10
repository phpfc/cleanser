//! Scheduler module for automated cleanup tasks.
//!
//! This module provides functionality to schedule and manage automatic
//! cleanup jobs using platform-specific schedulers:
//! - macOS: launchd (plist files)
//! - Linux: systemd user timers
//! - Windows: Task Scheduler (schtasks)

mod schedule;

#[cfg(target_os = "macos")]
mod launchd;

#[cfg(target_os = "linux")]
mod systemd;

#[cfg(target_os = "windows")]
mod windows;

pub use schedule::{ScheduleConfig, ScheduleFrequency, ScheduledJob, ScheduledJobResult};

use crate::platform::Platform;
use anyhow::Result;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Platform-agnostic scheduler manager
pub struct Scheduler {
    config: ScheduleConfig,
}

impl Scheduler {
    /// Create a new scheduler, loading existing configuration
    pub fn new() -> Result<Self> {
        let config = ScheduleConfig::load()?;
        Ok(Self { config })
    }

    /// Get the path to the cleanser binary
    fn get_cleanser_path() -> Result<PathBuf> {
        // Try to find the cleanser binary
        // First, check if we're running from the binary itself
        if let Ok(exe) = std::env::current_exe() {
            if exe.file_name().map(|n| n.to_str()) == Some(Some("cleanser")) {
                return Ok(exe);
            }
        }

        // Check common installation paths
        let common_paths = [
            "/usr/local/bin/cleanser",
            "/opt/homebrew/bin/cleanser",
            "/usr/bin/cleanser",
        ];

        for path in common_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Ok(p);
            }
        }

        // On Windows, check additional paths
        #[cfg(target_os = "windows")]
        {
            if let Ok(path) = which::which("cleanser") {
                return Ok(path);
            }
        }

        // Fall back to just "cleanser" and hope it's in PATH
        Ok(PathBuf::from("cleanser"))
    }

    /// Create and install a new scheduled job
    pub fn create_job(&mut self, job: ScheduledJob) -> Result<()> {
        // Validate job
        if job.name.is_empty() {
            anyhow::bail!("Job name cannot be empty");
        }

        // Check for duplicate names
        if self.config.jobs.iter().any(|j| j.name == job.name) {
            anyhow::bail!("A job with name '{}' already exists", job.name);
        }

        // Install on platform scheduler
        self.install_platform_job(&job)?;

        // Add to config
        self.config.add_job(job.clone());
        self.config.save()?;

        info!("Created scheduled job: {}", job.name);
        Ok(())
    }

    /// Remove a scheduled job by name or ID
    pub fn remove_job(&mut self, name_or_id: &str) -> Result<()> {
        let job = self
            .config
            .find_job(name_or_id)
            .ok_or_else(|| anyhow::anyhow!("Job not found: {}", name_or_id))?
            .clone();

        // Uninstall from platform scheduler
        self.uninstall_platform_job(&job)?;

        // Remove from config
        self.config.remove_job(&job.id);
        self.config.save()?;

        info!("Removed scheduled job: {}", job.name);
        Ok(())
    }

    /// Enable a job
    pub fn enable_job(&mut self, name_or_id: &str) -> Result<()> {
        let job = self
            .config
            .find_job_mut(name_or_id)
            .ok_or_else(|| anyhow::anyhow!("Job not found: {}", name_or_id))?;

        if job.enabled {
            return Ok(()); // Already enabled
        }

        job.enabled = true;
        let job_clone = job.clone();

        self.install_platform_job(&job_clone)?;
        self.config.save()?;

        info!("Enabled scheduled job: {}", job_clone.name);
        Ok(())
    }

    /// Disable a job
    pub fn disable_job(&mut self, name_or_id: &str) -> Result<()> {
        let job = self
            .config
            .find_job_mut(name_or_id)
            .ok_or_else(|| anyhow::anyhow!("Job not found: {}", name_or_id))?;

        if !job.enabled {
            return Ok(()); // Already disabled
        }

        job.enabled = false;
        let job_clone = job.clone();

        self.uninstall_platform_job(&job_clone)?;
        self.config.save()?;

        info!("Disabled scheduled job: {}", job_clone.name);
        Ok(())
    }

    /// List all jobs
    pub fn list_jobs(&self) -> &[ScheduledJob] {
        &self.config.jobs
    }

    /// Get a specific job
    pub fn get_job(&self, name_or_id: &str) -> Option<&ScheduledJob> {
        self.config.find_job(name_or_id)
    }

    /// Get job history
    pub fn get_history(&self, job_name: Option<&str>, limit: usize) -> Vec<&ScheduledJobResult> {
        let mut history: Vec<_> = self.config.history.iter().collect();

        if let Some(name) = job_name {
            if let Some(job) = self.config.find_job(name) {
                history.retain(|h| h.job_id == job.id);
            }
        }

        // Sort by start time, most recent first
        history.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        history.truncate(limit);
        history
    }

    /// Record a job result
    pub fn record_result(&mut self, result: ScheduledJobResult) -> Result<()> {
        // Update last_run on the job
        if let Some(job) = self.config.jobs.iter_mut().find(|j| j.id == result.job_id) {
            job.last_run = Some(result.completed_at);
        }

        self.config.add_result(result);
        self.config.save()?;
        Ok(())
    }

    /// Install a job on the platform scheduler
    fn install_platform_job(&self, job: &ScheduledJob) -> Result<()> {
        if !job.enabled {
            debug!("Skipping disabled job: {}", job.name);
            return Ok(());
        }

        let cleanser_path = Self::get_cleanser_path()?;

        match Platform::current() {
            #[cfg(target_os = "macos")]
            Platform::MacOS => launchd::install(job, &cleanser_path),
            #[cfg(target_os = "linux")]
            Platform::Linux => systemd::install(job, &cleanser_path),
            #[cfg(target_os = "windows")]
            Platform::Windows => windows::install(job, &cleanser_path),
            #[allow(unreachable_patterns)]
            _ => {
                warn!("Scheduler not implemented for this platform");
                Ok(())
            }
        }
    }

    /// Uninstall a job from the platform scheduler
    fn uninstall_platform_job(&self, job: &ScheduledJob) -> Result<()> {
        match Platform::current() {
            #[cfg(target_os = "macos")]
            Platform::MacOS => launchd::uninstall(job),
            #[cfg(target_os = "linux")]
            Platform::Linux => systemd::uninstall(job),
            #[cfg(target_os = "windows")]
            Platform::Windows => windows::uninstall(job),
            #[allow(unreachable_patterns)]
            _ => {
                warn!("Scheduler not implemented for this platform");
                Ok(())
            }
        }
    }

    /// Sync installed jobs with configuration
    pub fn sync(&mut self) -> Result<()> {
        for job in &self.config.jobs {
            if job.enabled {
                if let Err(e) = self.install_platform_job(job) {
                    warn!("Failed to sync job {}: {}", job.name, e);
                }
            }
        }
        Ok(())
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            config: ScheduleConfig::default(),
        })
    }
}
