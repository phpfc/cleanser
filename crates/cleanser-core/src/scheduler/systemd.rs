//! Linux systemd user timer implementation.

use super::schedule::{ScheduleFrequency, ScheduledJob};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info};

/// Get the systemd user unit directory
fn get_systemd_user_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;

    Ok(config_dir.join("systemd/user"))
}

/// Get the timer unit path for a job
fn get_timer_path(job: &ScheduledJob) -> Result<PathBuf> {
    let dir = get_systemd_user_dir()?;
    Ok(dir.join(format!("{}.timer", job.systemd_unit_name())))
}

/// Get the service unit path for a job
fn get_service_path(job: &ScheduledJob) -> Result<PathBuf> {
    let dir = get_systemd_user_dir()?;
    Ok(dir.join(format!("{}.service", job.systemd_unit_name())))
}

/// Generate timer unit content
fn generate_timer(job: &ScheduledJob) -> String {
    let on_calendar = generate_on_calendar(&job.frequency);

    format!(
        r#"[Unit]
Description=Cleanser scheduled cleanup: {name}

[Timer]
{on_calendar}
Persistent=true

[Install]
WantedBy=timers.target
"#,
        name = job.name,
        on_calendar = on_calendar,
    )
}

/// Generate service unit content
fn generate_service(job: &ScheduledJob, cleanser_path: &Path) -> String {
    let args = job.build_args().join(" ");

    format!(
        r#"[Unit]
Description=Cleanser cleanup job: {name}

[Service]
Type=oneshot
ExecStart={cleanser_path} {args}
Environment="PATH=/usr/local/bin:/usr/bin:/bin"

[Install]
WantedBy=default.target
"#,
        name = job.name,
        cleanser_path = cleanser_path.display(),
        args = args,
    )
}

/// Generate OnCalendar directive
fn generate_on_calendar(frequency: &ScheduleFrequency) -> String {
    match frequency {
        ScheduleFrequency::Hourly(n) => {
            format!("OnCalendar=*:0/{}", n * 60)
        }
        ScheduleFrequency::Daily { hour, minute } => {
            format!("OnCalendar=*-*-* {:02}:{:02}:00", hour, minute)
        }
        ScheduleFrequency::Weekly { days, hour, minute } => {
            let day_names: Vec<&str> = days
                .iter()
                .map(|d| match d {
                    chrono::Weekday::Mon => "Mon",
                    chrono::Weekday::Tue => "Tue",
                    chrono::Weekday::Wed => "Wed",
                    chrono::Weekday::Thu => "Thu",
                    chrono::Weekday::Fri => "Fri",
                    chrono::Weekday::Sat => "Sat",
                    chrono::Weekday::Sun => "Sun",
                })
                .collect();
            format!(
                "OnCalendar={} *-*-* {:02}:{:02}:00",
                day_names.join(","),
                hour,
                minute
            )
        }
        ScheduleFrequency::Monthly { day, hour, minute } => {
            format!("OnCalendar=*-*-{:02} {:02}:{:02}:00", day, hour, minute)
        }
        ScheduleFrequency::Cron(expr) => {
            // systemd doesn't use cron syntax, so we try to convert
            // For now, just use the expression and hope it works
            format!("OnCalendar={}", expr)
        }
    }
}

/// Install a job into systemd user timers
pub fn install(job: &ScheduledJob, cleanser_path: &Path) -> Result<()> {
    let user_dir = get_systemd_user_dir()?;
    fs::create_dir_all(&user_dir)?;

    let timer_path = get_timer_path(job)?;
    let service_path = get_service_path(job)?;

    let timer_content = generate_timer(job);
    let service_content = generate_service(job, cleanser_path);

    // Stop and disable if already exists
    let _ = stop_timer(job);
    let _ = disable_timer(job);

    // Write unit files
    fs::write(&timer_path, timer_content)
        .with_context(|| format!("Failed to write timer: {}", timer_path.display()))?;
    fs::write(&service_path, service_content)
        .with_context(|| format!("Failed to write service: {}", service_path.display()))?;

    // Reload systemd
    reload_daemon()?;

    // Enable and start timer
    enable_timer(job)?;
    start_timer(job)?;

    info!("Installed systemd timer: {}", job.name);
    debug!("Timer: {}", timer_path.display());

    Ok(())
}

/// Uninstall a job from systemd
pub fn uninstall(job: &ScheduledJob) -> Result<()> {
    let timer_path = get_timer_path(job)?;
    let service_path = get_service_path(job)?;

    // Stop and disable
    let _ = stop_timer(job);
    let _ = disable_timer(job);

    // Remove unit files
    if timer_path.exists() {
        fs::remove_file(&timer_path)?;
    }
    if service_path.exists() {
        fs::remove_file(&service_path)?;
    }

    // Reload daemon
    let _ = reload_daemon();

    info!("Uninstalled systemd timer: {}", job.name);
    Ok(())
}

/// Reload systemd user daemon
fn reload_daemon() -> Result<()> {
    let output = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output()
        .context("Failed to run systemctl daemon-reload")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        debug!("daemon-reload warning: {}", stderr);
    }

    Ok(())
}

/// Enable a timer
fn enable_timer(job: &ScheduledJob) -> Result<()> {
    let unit_name = format!("{}.timer", job.systemd_unit_name());

    let output = Command::new("systemctl")
        .args(["--user", "enable", &unit_name])
        .output()
        .context("Failed to enable timer")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to enable timer: {}", stderr);
    }

    Ok(())
}

/// Disable a timer
fn disable_timer(job: &ScheduledJob) -> Result<()> {
    let unit_name = format!("{}.timer", job.systemd_unit_name());

    let _ = Command::new("systemctl")
        .args(["--user", "disable", &unit_name])
        .output();

    Ok(())
}

/// Start a timer
fn start_timer(job: &ScheduledJob) -> Result<()> {
    let unit_name = format!("{}.timer", job.systemd_unit_name());

    let output = Command::new("systemctl")
        .args(["--user", "start", &unit_name])
        .output()
        .context("Failed to start timer")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to start timer: {}", stderr);
    }

    Ok(())
}

/// Stop a timer
fn stop_timer(job: &ScheduledJob) -> Result<()> {
    let unit_name = format!("{}.timer", job.systemd_unit_name());

    let _ = Command::new("systemctl")
        .args(["--user", "stop", &unit_name])
        .output();

    Ok(())
}

/// Check if a timer is active
#[allow(dead_code)]
pub fn is_active(job: &ScheduledJob) -> bool {
    let unit_name = format!("{}.timer", job.systemd_unit_name());

    let output = Command::new("systemctl")
        .args(["--user", "is-active", &unit_name])
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
    fn test_generate_timer_daily() {
        let job = ScheduledJob::new(
            "test".to_string(),
            ScheduleFrequency::Daily { hour: 9, minute: 30 },
        );

        let timer = generate_timer(&job);

        assert!(timer.contains("[Timer]"));
        assert!(timer.contains("OnCalendar=*-*-* 09:30:00"));
    }

    #[test]
    fn test_generate_service() {
        let job = ScheduledJob::new(
            "test".to_string(),
            ScheduleFrequency::Daily { hour: 9, minute: 0 },
        );

        let service = generate_service(&job, Path::new("/usr/bin/cleanser"));

        assert!(service.contains("[Service]"));
        assert!(service.contains("ExecStart=/usr/bin/cleanser"));
        assert!(service.contains("clean"));
    }
}
