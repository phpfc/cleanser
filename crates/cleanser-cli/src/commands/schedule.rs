//! Schedule management commands.

use cleanser_core::{RiskLevel, ScheduleFrequency, ScheduledJob, Scheduler};
use colored::Colorize;
use humansize::{format_size, BINARY};
use std::path::PathBuf;

/// Create a new scheduled job
pub fn set(
    name: String,
    frequency: String,
    risk: RiskLevel,
    paths: Vec<PathBuf>,
    trash: bool,
    secure: bool,
    secure_passes: u8,
    notify: bool,
) -> anyhow::Result<()> {
    let freq = ScheduleFrequency::parse(&frequency)?;

    let mut job = ScheduledJob::new(name.clone(), freq);
    job.risk_level = risk;
    job.paths = paths;
    job.use_trash = trash;
    job.secure_delete = secure;
    job.secure_delete_passes = secure_passes;
    job.notify_on_complete = notify;

    let mut scheduler = Scheduler::new()?;
    scheduler.create_job(job)?;

    println!("{} Created scheduled job: {}", "OK".green(), name);
    println!("  Schedule: {}", ScheduleFrequency::parse(&frequency)?.description());
    println!("  Risk level: {}", risk);

    if trash {
        println!("  Deletion: Move to trash");
    } else if secure {
        println!("  Deletion: Secure ({} passes)", secure_passes);
    } else {
        println!("  Deletion: Standard");
    }

    Ok(())
}

/// List all scheduled jobs
pub fn list(json: bool) -> anyhow::Result<()> {
    let scheduler = Scheduler::new()?;
    let jobs = scheduler.list_jobs();

    if jobs.is_empty() {
        println!("{}", "No scheduled jobs".yellow());
        println!("{}", "Create one with: cleanser schedule set <name> -f <frequency>".dimmed());
        return Ok(());
    }

    if json {
        println!("{}", serde_json::to_string_pretty(jobs)?);
        return Ok(());
    }

    println!("{}", "=== Scheduled Jobs ===".cyan().bold());
    println!();

    for job in jobs {
        let status = if job.enabled {
            "enabled".green()
        } else {
            "disabled".red()
        };

        println!("{} [{}]", job.name.bold(), status);
        println!("  ID: {}", job.id[..8].dimmed());
        println!("  Schedule: {}", job.frequency.description());
        println!("  Risk level: {}", job.risk_level);

        if job.use_trash {
            println!("  Deletion: Move to trash");
        } else if job.secure_delete {
            println!("  Deletion: Secure ({} passes)", job.secure_delete_passes);
        } else {
            println!("  Deletion: Standard");
        }

        if let Some(last_run) = job.last_run {
            println!("  Last run: {}", last_run.format("%Y-%m-%d %H:%M:%S"));
        } else {
            println!("  Last run: Never");
        }

        println!();
    }

    Ok(())
}

/// Remove a scheduled job
pub fn remove(job_name: String) -> anyhow::Result<()> {
    let mut scheduler = Scheduler::new()?;
    scheduler.remove_job(&job_name)?;

    println!("{} Removed scheduled job: {}", "OK".green(), job_name);
    Ok(())
}

/// Enable a scheduled job
pub fn enable(job_name: String) -> anyhow::Result<()> {
    let mut scheduler = Scheduler::new()?;
    scheduler.enable_job(&job_name)?;

    println!("{} Enabled scheduled job: {}", "OK".green(), job_name);
    Ok(())
}

/// Disable a scheduled job
pub fn disable(job_name: String) -> anyhow::Result<()> {
    let mut scheduler = Scheduler::new()?;
    scheduler.disable_job(&job_name)?;

    println!("{} Disabled scheduled job: {}", "OK".green(), job_name);
    Ok(())
}

/// Show job history
pub fn history(job_name: Option<String>, limit: usize) -> anyhow::Result<()> {
    let scheduler = Scheduler::new()?;
    let history = scheduler.get_history(job_name.as_deref(), limit);

    if history.is_empty() {
        println!("{}", "No job history".yellow());
        return Ok(());
    }

    println!("{}", "=== Job History ===".cyan().bold());
    println!();

    for result in history {
        let status = if result.error.is_some() {
            "FAILED".red()
        } else {
            "OK".green()
        };

        println!(
            "[{}] {} - {} items, {}",
            status,
            result.started_at.format("%Y-%m-%d %H:%M"),
            result.cleaned_count,
            format_size(result.cleaned_size, BINARY)
        );

        if let Some(ref error) = result.error {
            println!("  Error: {}", error.red());
        }
    }

    Ok(())
}

/// Run a job immediately
pub fn run(job_name: String, dry_run: bool) -> anyhow::Result<()> {
    let scheduler = Scheduler::new()?;

    let job = scheduler
        .get_job(&job_name)
        .ok_or_else(|| anyhow::anyhow!("Job not found: {}", job_name))?;

    println!("Running job: {}", job.name);
    println!("Schedule: {}", job.frequency.description());

    if dry_run {
        println!("{}", "DRY RUN - Would execute:".yellow());
    } else {
        println!("{}", "Executing:".cyan());
    }

    let args = job.build_args();
    println!("  cleanser {}", args.join(" "));

    if !dry_run {
        // Actually run the clean command
        use cleanser_core::{clean_with_config, CleanConfig, DeletionMethod, SecureDeleteConfig};
        use crate::progress::CliProgress;

        let deletion_method = if job.use_trash {
            DeletionMethod::Trash
        } else if job.secure_delete {
            DeletionMethod::Secure(SecureDeleteConfig::with_passes(job.secure_delete_passes))
        } else {
            DeletionMethod::Standard
        };

        let config = CleanConfig {
            max_risk: job.risk_level,
            dry_run: false,
            force_scan: false,
            deletion_method,
        };

        let progress = CliProgress::new_spinner();
        let result = clean_with_config(config, &progress)?;
        progress.finish();

        println!("\n{}", "=== Job Complete ===".green().bold());
        println!("Cleaned: {} items", result.cleaned_count);
        println!("Space freed: {}", format_size(result.cleaned_size, BINARY));

        if result.failed_count > 0 {
            println!("{}: {} items", "Failed".red(), result.failed_count);
        }
    }

    Ok(())
}
