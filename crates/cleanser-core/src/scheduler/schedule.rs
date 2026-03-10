//! Schedule configuration and types.

use crate::platform::Platform;
use crate::types::RiskLevel;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc, Weekday};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::PathBuf;
use tracing::debug;

/// Current config schema version
const CONFIG_VERSION: u32 = 1;

/// Maximum history entries to keep
const MAX_HISTORY_ENTRIES: usize = 100;

/// Schedule frequency configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleFrequency {
    /// Run every N hours
    Hourly(u8),
    /// Run daily at specified time
    Daily { hour: u8, minute: u8 },
    /// Run weekly on specified days at specified time
    Weekly {
        days: Vec<Weekday>,
        hour: u8,
        minute: u8,
    },
    /// Run monthly on specified day at specified time
    Monthly { day: u8, hour: u8, minute: u8 },
    /// Cron expression (Unix-style)
    Cron(String),
}

impl ScheduleFrequency {
    /// Parse a frequency from a human-readable string
    ///
    /// Formats:
    /// - "hourly" or "hourly@2" (every 2 hours)
    /// - "daily@09:00"
    /// - "weekly@Mon,Wed,Fri@14:30"
    /// - "monthly@15@09:00" (15th of each month at 09:00)
    /// - Cron expression: "0 9 * * *"
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim().to_lowercase();

        // Check for cron expression (starts with a number or *)
        if s.chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || c == '*')
        {
            return Ok(Self::Cron(s));
        }

        // Parse human-readable formats
        if s.starts_with("hourly") {
            let hours = if s.contains('@') {
                s.split('@')
                    .nth(1)
                    .and_then(|h| h.parse().ok())
                    .unwrap_or(1)
            } else {
                1
            };
            return Ok(Self::Hourly(hours));
        }

        if s.starts_with("daily") {
            let (hour, minute) = Self::parse_time(s.split('@').nth(1).unwrap_or("09:00"))?;
            return Ok(Self::Daily { hour, minute });
        }

        if s.starts_with("weekly") {
            let parts: Vec<&str> = s.split('@').collect();
            let days = if parts.len() > 1 {
                Self::parse_weekdays(parts[1])?
            } else {
                vec![Weekday::Mon]
            };
            let (hour, minute) = if parts.len() > 2 {
                Self::parse_time(parts[2])?
            } else {
                (9, 0)
            };
            return Ok(Self::Weekly { days, hour, minute });
        }

        if s.starts_with("monthly") {
            let parts: Vec<&str> = s.split('@').collect();
            let day = if parts.len() > 1 {
                parts[1].parse().unwrap_or(1)
            } else {
                1
            };
            let (hour, minute) = if parts.len() > 2 {
                Self::parse_time(parts[2])?
            } else {
                (9, 0)
            };
            return Ok(Self::Monthly { day, hour, minute });
        }

        anyhow::bail!("Invalid schedule format: {}", s);
    }

    /// Parse time string "HH:MM"
    fn parse_time(s: &str) -> Result<(u8, u8)> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid time format: {}", s);
        }

        let hour: u8 = parts[0].parse().context("Invalid hour")?;
        let minute: u8 = parts[1].parse().context("Invalid minute")?;

        if hour > 23 || minute > 59 {
            anyhow::bail!("Time out of range: {}:{}", hour, minute);
        }

        Ok((hour, minute))
    }

    /// Parse weekday string "Mon,Wed,Fri"
    fn parse_weekdays(s: &str) -> Result<Vec<Weekday>> {
        let mut days = Vec::new();

        for day_str in s.split(',') {
            let day = match day_str.trim().to_lowercase().as_str() {
                "mon" | "monday" => Weekday::Mon,
                "tue" | "tuesday" => Weekday::Tue,
                "wed" | "wednesday" => Weekday::Wed,
                "thu" | "thursday" => Weekday::Thu,
                "fri" | "friday" => Weekday::Fri,
                "sat" | "saturday" => Weekday::Sat,
                "sun" | "sunday" => Weekday::Sun,
                _ => anyhow::bail!("Invalid weekday: {}", day_str),
            };
            days.push(day);
        }

        if days.is_empty() {
            anyhow::bail!("At least one weekday is required");
        }

        Ok(days)
    }

    /// Convert to cron expression
    pub fn to_cron(&self) -> String {
        match self {
            Self::Hourly(n) => format!("0 */{} * * *", n),
            Self::Daily { hour, minute } => format!("{} {} * * *", minute, hour),
            Self::Weekly { days, hour, minute } => {
                let day_nums: Vec<String> = days
                    .iter()
                    .map(|d| {
                        match d {
                            Weekday::Sun => "0",
                            Weekday::Mon => "1",
                            Weekday::Tue => "2",
                            Weekday::Wed => "3",
                            Weekday::Thu => "4",
                            Weekday::Fri => "5",
                            Weekday::Sat => "6",
                        }
                        .to_string()
                    })
                    .collect();
                format!("{} {} * * {}", minute, hour, day_nums.join(","))
            }
            Self::Monthly { day, hour, minute } => format!("{} {} {} * *", minute, hour, day),
            Self::Cron(expr) => expr.clone(),
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> String {
        match self {
            Self::Hourly(1) => "Every hour".to_string(),
            Self::Hourly(n) => format!("Every {} hours", n),
            Self::Daily { hour, minute } => format!("Daily at {:02}:{:02}", hour, minute),
            Self::Weekly { days, hour, minute } => {
                let day_names: Vec<&str> = days
                    .iter()
                    .map(|d| match d {
                        Weekday::Mon => "Mon",
                        Weekday::Tue => "Tue",
                        Weekday::Wed => "Wed",
                        Weekday::Thu => "Thu",
                        Weekday::Fri => "Fri",
                        Weekday::Sat => "Sat",
                        Weekday::Sun => "Sun",
                    })
                    .collect();
                format!(
                    "Weekly on {} at {:02}:{:02}",
                    day_names.join(", "),
                    hour,
                    minute
                )
            }
            Self::Monthly { day, hour, minute } => {
                format!("Monthly on day {} at {:02}:{:02}", day, hour, minute)
            }
            Self::Cron(expr) => format!("Cron: {}", expr),
        }
    }
}

/// A scheduled cleanup job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    /// Unique identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// When this job runs
    pub frequency: ScheduleFrequency,
    /// Maximum risk level to clean
    pub risk_level: RiskLevel,
    /// Paths to scan (empty = default home)
    pub paths: Vec<PathBuf>,
    /// Whether to use secure delete
    pub secure_delete: bool,
    /// Secure delete passes (if enabled)
    pub secure_delete_passes: u8,
    /// Whether to use trash instead of permanent delete
    pub use_trash: bool,
    /// Whether job is enabled
    pub enabled: bool,
    /// When job was created
    pub created_at: DateTime<Utc>,
    /// Last time job ran
    pub last_run: Option<DateTime<Utc>>,
    /// Send notification on completion
    pub notify_on_complete: bool,
}

impl ScheduledJob {
    /// Create a new scheduled job
    pub fn new(name: String, frequency: ScheduleFrequency) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            frequency,
            risk_level: RiskLevel::Safe,
            paths: Vec::new(),
            secure_delete: false,
            secure_delete_passes: 3,
            use_trash: false,
            enabled: true,
            created_at: Utc::now(),
            last_run: None,
            notify_on_complete: false,
        }
    }

    /// Build command line arguments for this job
    pub fn build_args(&self) -> Vec<String> {
        let mut args = vec!["clean".to_string()];

        // Risk level
        args.push("--risk".to_string());
        args.push(self.risk_level.to_string().to_lowercase());

        // Skip confirmation
        args.push("-y".to_string());

        // Deletion method
        if self.use_trash {
            args.push("--trash".to_string());
        } else if self.secure_delete {
            args.push("--secure".to_string());
            args.push("--secure-passes".to_string());
            args.push(self.secure_delete_passes.to_string());
        }

        args
    }

    /// Get the launchd label for this job
    pub fn launchd_label(&self) -> String {
        format!("com.cleanser.job.{}", self.id.replace('-', ""))
    }

    /// Get the systemd unit name for this job
    pub fn systemd_unit_name(&self) -> String {
        format!("cleanser-{}", self.id.replace('-', ""))
    }
}

/// Result of a scheduled job run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJobResult {
    /// Job ID that ran
    pub job_id: String,
    /// When the job started
    pub started_at: DateTime<Utc>,
    /// When the job completed
    pub completed_at: DateTime<Utc>,
    /// Number of items cleaned
    pub cleaned_count: usize,
    /// Total bytes cleaned
    pub cleaned_size: u64,
    /// Number of failures
    pub failed_count: usize,
    /// Error message if job failed
    pub error: Option<String>,
}

/// Schedule configuration storage
#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleConfig {
    /// Schema version
    pub version: u32,
    /// All scheduled jobs
    pub jobs: Vec<ScheduledJob>,
    /// Job run history
    pub history: Vec<ScheduledJobResult>,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            jobs: Vec::new(),
            history: Vec::new(),
        }
    }
}

impl ScheduleConfig {
    /// Get the config file path
    fn get_config_path() -> Result<PathBuf> {
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

        Ok(config_dir.join("schedules.json"))
    }

    /// Get the lock file path
    fn get_lock_path() -> Result<PathBuf> {
        let config_path = Self::get_config_path()?;
        Ok(config_path.with_extension("lock"))
    }

    /// Load configuration from disk
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        // Acquire shared lock
        let lock_path = Self::get_lock_path()?;
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let lock_file = File::create(&lock_path)?;
        lock_file.lock_shared()?;

        let content = fs::read_to_string(&config_path)?;
        let config: Self = serde_json::from_str(&content)?;

        lock_file.unlock()?;

        debug!("Loaded schedule config with {} jobs", config.jobs.len());
        Ok(config)
    }

    /// Save configuration to disk
    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Acquire exclusive lock
        let lock_path = Self::get_lock_path()?;
        let lock_file = File::create(&lock_path)?;
        lock_file.lock_exclusive()?;

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, content)?;

        lock_file.unlock()?;

        debug!("Saved schedule config with {} jobs", self.jobs.len());
        Ok(())
    }

    /// Add a job
    pub fn add_job(&mut self, job: ScheduledJob) {
        self.jobs.push(job);
    }

    /// Remove a job by ID
    pub fn remove_job(&mut self, id: &str) -> bool {
        if let Some(pos) = self.jobs.iter().position(|j| j.id == id) {
            self.jobs.remove(pos);
            true
        } else {
            false
        }
    }

    /// Find a job by name or ID
    pub fn find_job(&self, name_or_id: &str) -> Option<&ScheduledJob> {
        self.jobs
            .iter()
            .find(|j| j.name == name_or_id || j.id == name_or_id || j.id.starts_with(name_or_id))
    }

    /// Find a job by name or ID (mutable)
    pub fn find_job_mut(&mut self, name_or_id: &str) -> Option<&mut ScheduledJob> {
        self.jobs
            .iter_mut()
            .find(|j| j.name == name_or_id || j.id == name_or_id || j.id.starts_with(name_or_id))
    }

    /// Add a result to history
    pub fn add_result(&mut self, result: ScheduledJobResult) {
        self.history.push(result);

        // Trim history if too large
        if self.history.len() > MAX_HISTORY_ENTRIES {
            self.history
                .drain(0..self.history.len() - MAX_HISTORY_ENTRIES);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hourly() {
        let freq = ScheduleFrequency::parse("hourly").unwrap();
        assert_eq!(freq, ScheduleFrequency::Hourly(1));

        let freq = ScheduleFrequency::parse("hourly@2").unwrap();
        assert_eq!(freq, ScheduleFrequency::Hourly(2));
    }

    #[test]
    fn test_parse_daily() {
        let freq = ScheduleFrequency::parse("daily@09:30").unwrap();
        assert_eq!(
            freq,
            ScheduleFrequency::Daily {
                hour: 9,
                minute: 30
            }
        );
    }

    #[test]
    fn test_parse_weekly() {
        let freq = ScheduleFrequency::parse("weekly@Mon,Wed,Fri@14:00").unwrap();
        match freq {
            ScheduleFrequency::Weekly { days, hour, minute } => {
                assert_eq!(days.len(), 3);
                assert_eq!(hour, 14);
                assert_eq!(minute, 0);
            }
            _ => panic!("Expected Weekly"),
        }
    }

    #[test]
    fn test_parse_cron() {
        let freq = ScheduleFrequency::parse("0 9 * * 1-5").unwrap();
        assert_eq!(freq, ScheduleFrequency::Cron("0 9 * * 1-5".to_string()));
    }

    #[test]
    fn test_to_cron() {
        let freq = ScheduleFrequency::Daily {
            hour: 9,
            minute: 30,
        };
        assert_eq!(freq.to_cron(), "30 9 * * *");

        let freq = ScheduleFrequency::Hourly(2);
        assert_eq!(freq.to_cron(), "0 */2 * * *");
    }

    #[test]
    fn test_scheduled_job_build_args() {
        let job = ScheduledJob::new(
            "test".to_string(),
            ScheduleFrequency::Daily { hour: 9, minute: 0 },
        );
        let args = job.build_args();

        assert!(args.contains(&"clean".to_string()));
        assert!(args.contains(&"--risk".to_string()));
        assert!(args.contains(&"-y".to_string()));
    }

    #[test]
    fn test_frequency_description() {
        let freq = ScheduleFrequency::Daily { hour: 9, minute: 0 };
        assert_eq!(freq.description(), "Daily at 09:00");

        let freq = ScheduleFrequency::Hourly(1);
        assert_eq!(freq.description(), "Every hour");
    }
}
