//! macOS launchd scheduler implementation.

use super::schedule::{ScheduleFrequency, ScheduledJob};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info};

/// Get the LaunchAgents directory
fn get_launch_agents_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    Ok(home.join("Library/LaunchAgents"))
}

/// Get the plist path for a job
fn get_plist_path(job: &ScheduledJob) -> Result<PathBuf> {
    let dir = get_launch_agents_dir()?;
    Ok(dir.join(format!("{}.plist", job.launchd_label())))
}

/// Generate plist content for a job
fn generate_plist(job: &ScheduledJob, cleanser_path: &Path) -> String {
    let args = job.build_args();
    let args_xml: String = args
        .iter()
        .map(|a| format!("        <string>{}</string>", a))
        .collect::<Vec<_>>()
        .join("\n");

    let calendar_interval = generate_calendar_interval(&job.frequency);

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>

    <key>ProgramArguments</key>
    <array>
        <string>{cleanser_path}</string>
{args}
    </array>

{calendar_interval}

    <key>StandardOutPath</key>
    <string>/tmp/cleanser-{id}.out</string>

    <key>StandardErrorPath</key>
    <string>/tmp/cleanser-{id}.err</string>

    <key>RunAtLoad</key>
    <false/>

    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin</string>
    </dict>
</dict>
</plist>
"#,
        label = job.launchd_label(),
        cleanser_path = cleanser_path.display(),
        args = args_xml,
        calendar_interval = calendar_interval,
        id = &job.id[..8],
    )
}

/// Generate the StartCalendarInterval section
fn generate_calendar_interval(frequency: &ScheduleFrequency) -> String {
    match frequency {
        ScheduleFrequency::Hourly(n) => {
            // For hourly, we use StartInterval instead of StartCalendarInterval
            let seconds = (*n as u32) * 3600;
            format!(
                r#"    <key>StartInterval</key>
    <integer>{}</integer>"#,
                seconds
            )
        }
        ScheduleFrequency::Daily { hour, minute } => {
            format!(
                r#"    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>{}</integer>
        <key>Minute</key>
        <integer>{}</integer>
    </dict>"#,
                hour, minute
            )
        }
        ScheduleFrequency::Weekly { days, hour, minute } => {
            let intervals: String = days
                .iter()
                .map(|day| {
                    let day_num = match day {
                        chrono::Weekday::Sun => 0,
                        chrono::Weekday::Mon => 1,
                        chrono::Weekday::Tue => 2,
                        chrono::Weekday::Wed => 3,
                        chrono::Weekday::Thu => 4,
                        chrono::Weekday::Fri => 5,
                        chrono::Weekday::Sat => 6,
                    };
                    format!(
                        r#"        <dict>
            <key>Weekday</key>
            <integer>{}</integer>
            <key>Hour</key>
            <integer>{}</integer>
            <key>Minute</key>
            <integer>{}</integer>
        </dict>"#,
                        day_num, hour, minute
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            format!(
                r#"    <key>StartCalendarInterval</key>
    <array>
{}
    </array>"#,
                intervals
            )
        }
        ScheduleFrequency::Monthly { day, hour, minute } => {
            format!(
                r#"    <key>StartCalendarInterval</key>
    <dict>
        <key>Day</key>
        <integer>{}</integer>
        <key>Hour</key>
        <integer>{}</integer>
        <key>Minute</key>
        <integer>{}</integer>
    </dict>"#,
                day, hour, minute
            )
        }
        ScheduleFrequency::Cron(_) => {
            // For cron expressions, we'd need to parse and convert
            // For now, default to daily at 9am
            r#"    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>9</integer>
        <key>Minute</key>
        <integer>0</integer>
    </dict>"#
                .to_string()
        }
    }
}

/// Install a job into launchd
pub fn install(job: &ScheduledJob, cleanser_path: &Path) -> Result<()> {
    let launch_agents = get_launch_agents_dir()?;
    fs::create_dir_all(&launch_agents)?;

    let plist_path = get_plist_path(job)?;
    let plist_content = generate_plist(job, cleanser_path);

    // Unload if already loaded
    let _ = unload(&plist_path);

    // Write plist
    fs::write(&plist_path, plist_content)
        .with_context(|| format!("Failed to write plist to {}", plist_path.display()))?;

    // Load the job
    load(&plist_path)?;

    info!("Installed launchd job: {}", job.name);
    debug!("Plist: {}", plist_path.display());

    Ok(())
}

/// Uninstall a job from launchd
pub fn uninstall(job: &ScheduledJob) -> Result<()> {
    let plist_path = get_plist_path(job)?;

    if plist_path.exists() {
        // Unload first
        let _ = unload(&plist_path);

        // Remove plist file
        fs::remove_file(&plist_path)
            .with_context(|| format!("Failed to remove plist: {}", plist_path.display()))?;
    }

    info!("Uninstalled launchd job: {}", job.name);
    Ok(())
}

/// Load a launchd job
fn load(plist_path: &Path) -> Result<()> {
    let output = Command::new("launchctl")
        .args(["load", "-w"])
        .arg(plist_path)
        .output()
        .context("Failed to run launchctl load")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("launchctl load failed: {}", stderr);
    }

    debug!("Loaded: {}", plist_path.display());
    Ok(())
}

/// Unload a launchd job
fn unload(plist_path: &Path) -> Result<()> {
    let output = Command::new("launchctl")
        .args(["unload", "-w"])
        .arg(plist_path)
        .output()
        .context("Failed to run launchctl unload")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        debug!("launchctl unload: {}", stderr);
    }

    Ok(())
}

/// Check if a job is loaded
#[allow(dead_code)]
pub fn is_loaded(job: &ScheduledJob) -> bool {
    let output = Command::new("launchctl")
        .args(["list", &job.launchd_label()])
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
    fn test_generate_plist_daily() {
        let job = ScheduledJob::new(
            "test".to_string(),
            ScheduleFrequency::Daily {
                hour: 9,
                minute: 30,
            },
        );

        let plist = generate_plist(&job, Path::new("/usr/local/bin/cleanser"));

        assert!(plist.contains("<key>Label</key>"));
        assert!(plist.contains("<key>StartCalendarInterval</key>"));
        assert!(plist.contains("<integer>9</integer>"));
        assert!(plist.contains("<integer>30</integer>"));
    }

    #[test]
    fn test_generate_plist_hourly() {
        let job = ScheduledJob::new("test".to_string(), ScheduleFrequency::Hourly(2));

        let plist = generate_plist(&job, Path::new("/usr/local/bin/cleanser"));

        assert!(plist.contains("<key>StartInterval</key>"));
        assert!(plist.contains("<integer>7200</integer>")); // 2 * 3600
    }
}
