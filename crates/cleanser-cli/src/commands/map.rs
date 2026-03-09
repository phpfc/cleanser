use cleanser_core::{DirectoryCategory, FileSystemCrawler, FileSystemMap, WhitelistConfig};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

/// Crawler progress callback for CLI
struct CliCrawlerProgress {
    pb: ProgressBar,
}

impl CliCrawlerProgress {
    fn new() -> Self {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap(),
        );
        Self { pb }
    }
}

impl cleanser_core::CrawlerProgress for CliCrawlerProgress {
    fn on_start(&self, message: &str) {
        self.pb.set_message(message.to_string());
    }

    fn on_progress(&self, current: usize, total: usize, path: &Path) {
        self.pb.set_message(format!(
            "Scanning {}/{}: {}",
            current,
            total,
            path.display()
        ));
    }

    fn on_complete(&self, message: &str) {
        self.pb.finish_with_message(message.to_string());
    }
}

pub fn show() -> anyhow::Result<()> {
    match FileSystemMap::load() {
        Ok(map) => {
            // Header
            println!();
            println!(
                "  {}",
                "╔════════════════════════════════════════════════════════════╗".cyan()
            );
            println!(
                "  {}  {:<56}  {}",
                "║".cyan(),
                "FILESYSTEM MAP".cyan().bold(),
                "║".cyan()
            );
            println!(
                "  {}",
                "╚════════════════════════════════════════════════════════════╝".cyan()
            );
            println!();

            // Summary stats
            let cleanable_count = map
                .directories
                .values()
                .filter(|d| {
                    matches!(
                        d.category,
                        DirectoryCategory::Ephemeral | DirectoryCategory::BuildArtifact
                    )
                })
                .count();

            println!(
                "  {} {} directories",
                "Total mapped:".dimmed(),
                map.total_directories
            );
            println!(
                "  {} {}",
                "Cleanable:".green().bold(),
                format!("{} directories", cleanable_count).green()
            );

            let created =
                chrono::DateTime::from_timestamp(map.created_at as i64, 0).unwrap_or_default();
            println!(
                "  {} {}",
                "Last scan:".dimmed(),
                created.format("%Y-%m-%d %H:%M")
            );

            if map.is_stale() {
                println!();
                println!(
                    "  {}",
                    "⚠ Map is stale (>7 days). Run: cleanser map rebuild".yellow()
                );
            }

            // Cleanable by category
            println!();
            println!(
                "  {}",
                "─── Cleanable Categories ──────────────────────────────────".cyan()
            );
            println!();

            let mut tag_counts: Vec<_> = map.stats_by_tag().into_iter().collect();
            tag_counts.sort_by(|a, b| b.1 .0.cmp(&a.1 .0)); // Sort by count

            let max_count = tag_counts.first().map(|(_, (c, _))| *c).unwrap_or(1);

            for (tag, (count, _)) in tag_counts.iter().take(10) {
                let bar_width = ((*count * 30) / max_count).max(1);
                let bar = "█".repeat(bar_width);

                println!("  {:>14}  {:>5} dirs  {}", tag.yellow(), count, bar.green());
            }

            println!();
            println!(
                "  {}",
                "─── Quick Actions ─────────────────────────────────────────".cyan()
            );
            println!();
            println!(
                "  {} cleanser scan              {}",
                "→".green(),
                "Scan and calculate sizes".dimmed()
            );
            println!(
                "  {} cleanser map stats         {}",
                "→".green(),
                "View detailed breakdown".dimmed()
            );
            println!(
                "  {} cleanser map rebuild       {}",
                "→".green(),
                "Refresh the map".dimmed()
            );
            println!();
        }
        Err(_) => {
            println!();
            println!("  {}", "No filesystem map found.".yellow());
            println!();
            println!("  Create one with: {}", "cleanser map rebuild".cyan());
            println!();
        }
    }
    Ok(())
}

pub fn rebuild(max_depth: usize, min_confidence: f32) -> anyhow::Result<()> {
    println!();
    println!("  {}", "Rebuilding filesystem map...".cyan());
    println!();

    let progress = Box::new(CliCrawlerProgress::new());
    let crawler = FileSystemCrawler::new()
        .with_max_depth(max_depth)
        .with_min_confidence(min_confidence)
        .with_progress_callback(progress);

    let map = crawler.crawl_full()?;
    map.save()?;

    let cleanable_count = map
        .directories
        .values()
        .filter(|d| {
            matches!(
                d.category,
                DirectoryCategory::Ephemeral | DirectoryCategory::BuildArtifact
            )
        })
        .count();

    println!();
    println!("  {}", "✓ Filesystem map rebuilt!".green().bold());
    println!();
    println!(
        "  {} {}",
        "Directories mapped:".dimmed(),
        map.total_directories
    );
    println!(
        "  {} {}",
        "Cleanable directories:".green(),
        format!("{}", cleanable_count).green().bold()
    );
    println!();
    println!("  {}", "Run 'cleanser scan' to analyze sizes.".dimmed());
    println!();
    Ok(())
}

pub fn stats() -> anyhow::Result<()> {
    match FileSystemMap::load() {
        Ok(map) => {
            println!();
            println!(
                "  {}",
                "╔════════════════════════════════════════════════════════════╗".cyan()
            );
            println!(
                "  {}  {:<56}  {}",
                "║".cyan(),
                "DETAILED STATISTICS".cyan().bold(),
                "║".cyan()
            );
            println!(
                "  {}",
                "╚════════════════════════════════════════════════════════════╝".cyan()
            );
            println!();

            // By Category with visual bars
            println!(
                "  {}",
                "─── By Category ───────────────────────────────────────────".cyan()
            );
            println!();

            let stats = map.stats_by_category();
            let mut stats_vec: Vec<_> = stats.iter().collect();
            stats_vec.sort_by(|a, b| b.1 .0.cmp(&a.1 .0)); // Sort by count
            let max_count = stats_vec.first().map(|(_, (c, _))| *c).unwrap_or(1);

            for (category, (count, _)) in &stats_vec {
                let cat_name = match category {
                    DirectoryCategory::Ephemeral => "Cache/Temp",
                    DirectoryCategory::BuildArtifact => "Build Artifacts",
                    DirectoryCategory::ApplicationData => "App Data",
                    DirectoryCategory::UserContent => "User Content",
                    DirectoryCategory::System => "System",
                    DirectoryCategory::Unknown => "Other",
                };
                let bar_width = ((*count * 25) / max_count).max(1);
                let bar = "█".repeat(bar_width);

                type ColorFn = fn(&str) -> colored::ColoredString;
                let (cat_color, bar_color): (ColorFn, ColorFn) = match category {
                    DirectoryCategory::Ephemeral => (|s| s.green(), |s| s.green()),
                    DirectoryCategory::BuildArtifact => (|s| s.yellow(), |s| s.yellow()),
                    DirectoryCategory::UserContent => (|s| s.blue(), |s| s.blue()),
                    _ => (|s| s.white(), |s| s.white()),
                };

                println!(
                    "  {:>16}  {:>6} dirs  {}",
                    cat_color(cat_name),
                    count,
                    bar_color(&bar)
                );
            }

            // By Tag
            println!();
            println!(
                "  {}",
                "─── By Type (Top 15) ──────────────────────────────────────".cyan()
            );
            println!();

            let tag_stats = map.stats_by_tag();
            let mut tag_vec: Vec<_> = tag_stats.iter().collect();
            tag_vec.sort_by(|a, b| b.1 .0.cmp(&a.1 .0)); // Sort by count

            for (tag, (count, _)) in tag_vec.iter().take(15) {
                println!("  {:>16}  {:>6} dirs", tag.yellow(), count);
            }

            // Confidence breakdown
            println!();
            println!(
                "  {}",
                "─── Classification Confidence ─────────────────────────────".cyan()
            );
            println!();

            let high_conf = map
                .directories
                .values()
                .filter(|d| d.confidence >= 0.9)
                .count();
            let med_conf = map
                .directories
                .values()
                .filter(|d| d.confidence >= 0.7 && d.confidence < 0.9)
                .count();
            let low_conf = map
                .directories
                .values()
                .filter(|d| d.confidence < 0.7)
                .count();
            let total = map.directories.len();

            println!(
                "  {:>16}  {} {}",
                "High (≥90%)".green(),
                format!("{:>5}", high_conf).green(),
                format!("({}%)", high_conf * 100 / total.max(1)).dimmed()
            );
            println!(
                "  {:>16}  {} {}",
                "Medium (70-90%)".yellow(),
                format!("{:>5}", med_conf).yellow(),
                format!("({}%)", med_conf * 100 / total.max(1)).dimmed()
            );
            println!(
                "  {:>16}  {} {}",
                "Low (<70%)".red(),
                format!("{:>5}", low_conf).red(),
                format!("({}%)", low_conf * 100 / total.max(1)).dimmed()
            );

            // Sample cleanable paths
            println!();
            println!(
                "  {}",
                "─── Sample Cleanable Paths ────────────────────────────────".cyan()
            );
            println!();

            let cleanable: Vec<_> = map
                .directories
                .values()
                .filter(|d| {
                    matches!(
                        d.category,
                        DirectoryCategory::Ephemeral | DirectoryCategory::BuildArtifact
                    )
                })
                .filter(|d| d.confidence >= 0.8)
                .take(8)
                .collect();

            if cleanable.is_empty() {
                println!(
                    "  {}",
                    "No high-confidence cleanable directories found.".dimmed()
                );
            } else {
                for dir in cleanable {
                    let path_str = dir.path.to_string_lossy();
                    let display_path = if path_str.len() > 55 {
                        format!("...{}", &path_str[path_str.len() - 52..])
                    } else {
                        path_str.to_string()
                    };

                    let tag = dir.tags.first().map(|s| s.as_str()).unwrap_or("");
                    println!(
                        "  {} {:55} {}",
                        "•".green(),
                        display_path,
                        format!("[{}]", tag).dimmed()
                    );
                }
            }

            println!();
            println!("  {}", "Run 'cleanser scan' to calculate sizes.".dimmed());
            println!();
        }
        Err(e) => {
            println!("{}", format!("No filesystem map found: {}", e).yellow());
            println!("{}", "Run 'cleanser map rebuild' to create one.".cyan());
        }
    }
    Ok(())
}

pub fn verify() -> anyhow::Result<()> {
    match FileSystemMap::load() {
        Ok(mut map) => {
            println!();
            println!("  {}", "Verifying filesystem map...".cyan());

            let total_dirs = map.directories.len();
            let invalid: Vec<_> = map
                .directories
                .iter()
                .filter(|(path, _)| !path.exists())
                .map(|(path, _)| path.clone())
                .collect();

            if invalid.is_empty() {
                println!("  {}", "✓ All directories still exist.".green());
            } else {
                println!("  {} {} invalid entries", "Found".yellow(), invalid.len());

                map.cleanup_invalid();
                map.save()?;

                println!(
                    "  {} Removed {} entries, {} remain.",
                    "✓".green(),
                    invalid.len(),
                    total_dirs - invalid.len()
                );
            }
            println!();
        }
        Err(e) => {
            println!("{}", format!("No filesystem map found: {}", e).yellow());
        }
    }
    Ok(())
}

pub fn suggest() -> anyhow::Result<()> {
    match FileSystemMap::load() {
        Ok(map) => {
            println!();
            println!(
                "  {}",
                "─── Whitelist Suggestions ─────────────────────────────────".cyan()
            );
            println!();

            let whitelist = WhitelistConfig::load()?;

            let suggestions: Vec<_> = map
                .directories
                .values()
                .filter(|d| {
                    d.confidence >= 0.95
                        && !whitelist.contains(&d.path)
                        && matches!(
                            d.category,
                            DirectoryCategory::System | DirectoryCategory::UserContent
                        )
                })
                .collect();

            if suggestions.is_empty() {
                println!(
                    "  {}",
                    "No suggestions - your whitelist looks comprehensive!".green()
                );
            } else {
                println!(
                    "  Found {} directories that should probably be protected:\n",
                    suggestions.len()
                );

                for dir in suggestions.iter().take(10) {
                    println!("  {} {}", "→".yellow(), dir.path.display());
                }

                println!();
                println!("  Add with: {}", "cleanser whitelist add <path>".cyan());
            }
            println!();
        }
        Err(e) => {
            println!("{}", format!("No filesystem map found: {}", e).yellow());
        }
    }
    Ok(())
}
