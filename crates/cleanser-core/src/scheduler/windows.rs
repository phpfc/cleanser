//! Windows Task Scheduler implementation.

use super::schedule::{ScheduleFrequency, ScheduledJob};
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;
use tracing::{debug, info};

/// Get the task name for a job
fn get_task_name(job: &ScheduledJob) -> String {
    format!("Cleanser-{}", &job.id[..8])
}

/// Install a job into Windows Task Scheduler
pub fn install(job: &ScheduledJob, cleanser_path: &Path) -> Result<()> {
    let task_name = get_task_name(job);
    let args = job.build_args().join(" ");

    // First, try to delete any existing task
    let _ = delete_task(&task_name);

    // Build schtasks command
    let (schedule_type, schedule_args) = build_schedule_args(&job.frequency);

    let mut cmd = Command::new("schtasks");
    cmd.args([
        "/create",
        "/tn",
        &task_name,
        "/tr",
        &format!("\"{}\" {}", cleanser_path.display(), args),
        "/sc",
        &schedule_type,
    ]);

    // Add schedule-specific arguments
    for arg in schedule_args {
        cmd.arg(arg);
    }

    // Don't require the computer to be idle
    cmd.arg("/f"); // Force create (overwrite if exists)

    let output = cmd.output().context("Failed to run schtasks")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("schtasks failed: {}", stderr);
    }

    info!("Installed Windows scheduled task: {}", job.name);
    debug!("Task name: {}", task_name);

    Ok(())
}

/// Uninstall a job from Windows Task Scheduler
pub fn uninstall(job: &ScheduledJob) -> Result<()> {
    let task_name = get_task_name(job);
    delete_task(&task_name)?;

    info!("Uninstalled Windows scheduled task: {}", job.name);
    Ok(())
}

/// Delete a scheduled task
fn delete_task(task_name: &str) -> Result<()> {
    let output = Command::new("schtasks")
        .args(["/delete", "/tn", task_name, "/f"])
        .output()
        .context("Failed to run schtasks delete")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        debug!("schtasks delete: {}", stderr);
    }

    Ok(())
}

/// Build schedule type and additional arguments for schtasks
fn build_schedule_args(frequency: &ScheduleFrequency) -> (String, Vec<String>) {
    match frequency {
        ScheduleFrequency::Hourly(n) => {
            let minutes = *n as u32 * 60;
            ("minute".to_string(), vec!["/mo".to_string(), minutes.to_string()])
        }
        ScheduleFrequency::Daily { hour, minute } => (
            "daily".to_string(),
            vec!["/st".to_string(), format!("{:02}:{:02}", hour, minute)],
        ),
        ScheduleFrequency::Weekly { days, hour, minute } => {
            let day_names: Vec<&str> = days
                .iter()
                .map(|d| match d {
                    chrono::Weekday::Mon => "MON",
                    chrono::Weekday::Tue => "TUE",
                    chrono::Weekday::Wed => "WED",
                    chrono::Weekday::Thu => "THU",
                    chrono::Weekday::Fri => "FRI",
                    chrono::Weekday::Sat => "SAT",
                    chrono::Weekday::Sun => "SUN",
                })
                .collect();
            (
                "weekly".to_string(),
                vec![
                    "/d".to_string(),
                    day_names.join(","),
                    "/st".to_string(),
                    format!("{:02}:{:02}", hour, minute),
                ],
            )
        }
        ScheduleFrequency::Monthly { day, hour, minute } => (
            "monthly".to_string(),
            vec![
                "/d".to_string(),
                day.to_string(),
                "/st".to_string(),
                format!("{:02}:{:02}", hour, minute),
            ],
        ),
        ScheduleFrequency::Cron(_) => {
            // Windows doesn't support cron, default to daily at 9am
            ("daily".to_string(), vec!["/st".to_string(), "09:00".to_string()])
        }
    }
}

/// Check if a task exists
#[allow(dead_code)]
pub fn task_exists(job: &ScheduledJob) -> bool {
    let task_name = get_task_name(job);

    let output = Command::new("schtasks")
        .args(["/query", "/tn", &task_name])
        .output();

    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_schedule_args_daily() {
        let (schedule_type, args) = build_schedule_args(&ScheduleFrequency::Daily {
            hour: 9,
            minute: 30,
        });

        assert_eq!(schedule_type, "daily");
        assert!(args.contains(&"/st".to_string()));
        assert!(args.contains(&"09:30".to_string()));
    }

    #[test]
    fn test_build_schedule_args_weekly() {
        let (schedule_type, args) = build_schedule_args(&ScheduleFrequency::Weekly {
            days: vec![chrono::Weekday::Mon, chrono::Weekday::Fri],
            hour: 14,
            minute: 0,
        });

        assert_eq!(schedule_type, "weekly");
        assert!(args.contains(&"/d".to_string()));
    }

    #[test]
    fn test_get_task_name() {
        let job = ScheduledJob::new(
            "test".to_string(),
            ScheduleFrequency::Daily { hour: 9, minute: 0 },
        );

        let name = get_task_name(&job);
        assert!(name.starts_with("Cleanser-"));
    }
}
