use crate::mapper::filesystem_map::DirectoryCategory;
use crate::platform;
use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// Get the setup marker file path
fn get_setup_marker_path() -> Result<PathBuf> {
    let home = platform::home_dir_or_err()?;
    let config_dir = match platform::Platform::current() {
        platform::Platform::MacOS => home.join("Library").join("Application Support").join("cleanser"),
        platform::Platform::Linux => home.join(".config").join("cleanser"),
        platform::Platform::Windows => home.join("AppData").join("Local").join("cleanser"),
    };
    Ok(config_dir.join(".setup_complete"))
}

/// Check if this is the first run
pub fn is_first_run() -> bool {
    match get_setup_marker_path() {
        Ok(path) => !path.exists(),
        Err(_) => false, // If we can't determine, assume not first run
    }
}

/// Mark setup as complete
fn mark_setup_complete() -> Result<()> {
    let path = get_setup_marker_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, "1")?;
    Ok(())
}

/// Run first-time setup wizard
pub fn run_first_time_setup() -> Result<()> {
    let platform = platform::Platform::current();

    println!();
    println!("{}", "╔════════════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║           Welcome to Cleanser!                             ║".cyan());
    println!("{}", "╚════════════════════════════════════════════════════════════╝".cyan());
    println!();
    println!("  Detected platform: {}", platform.name().green().bold());
    println!();
    println!("  Cleanser helps you free up disk space by finding and removing:");
    println!("    • System and application caches");
    println!("    • Build artifacts (node_modules, target/, dist/, etc.)");
    println!("    • Log files and temporary data");
    println!();

    // Ask about building filesystem map
    print!("  {} ", "Build filesystem map for faster scans? [Y/n]:".yellow());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let build_map = input.trim().is_empty() || input.trim().to_lowercase().starts_with('y');

    if build_map {
        println!();
        println!("  {}", "Building filesystem map...".cyan());

        let crawler = crate::mapper::FileSystemCrawler::new()
            .with_max_depth(6)
            .with_progress(true);

        let fs_map = crawler.crawl_full()?;
        fs_map.save()?;

        let cleanable_count = fs_map.directories.values()
            .filter(|d| d.category == DirectoryCategory::Ephemeral
                || d.category == DirectoryCategory::BuildArtifact)
            .count();

        println!("  {} Mapped {} directories ({} cleanable)",
            "✓".green(),
            fs_map.directories.len(),
            cleanable_count
        );
    }

    // Ask about common exclusions
    println!();
    print!("  {} ", "Add common directories to whitelist? [y/N]:".yellow());
    io::stdout().flush()?;

    input.clear();
    io::stdin().read_line(&mut input)?;
    let add_whitelist = input.trim().to_lowercase().starts_with('y');

    if add_whitelist {
        let mut whitelist = crate::types::WhitelistConfig::load().unwrap_or_else(|_| crate::types::WhitelistConfig::new());

        let home = platform::home_dir_or_err()?;
        let suggestions = get_whitelist_suggestions(&home, platform);

        println!();
        println!("  Common directories to protect:");
        for (i, (path, desc)) in suggestions.iter().enumerate() {
            if path.exists() {
                println!("    {}. {} - {}", i + 1, path.display(), desc.dimmed());
            }
        }

        print!("  {} ", "Enter numbers to add (e.g., 1,2,3) or 'all' or press Enter to skip:".yellow());
        io::stdout().flush()?;

        input.clear();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if !input.is_empty() {
            let indices: Vec<usize> = if input.to_lowercase() == "all" {
                (0..suggestions.len()).collect()
            } else {
                input.split(',')
                    .filter_map(|s| s.trim().parse::<usize>().ok())
                    .filter(|&i| i > 0 && i <= suggestions.len())
                    .map(|i| i - 1)
                    .collect()
            };

            for i in indices {
                if let Some((path, _)) = suggestions.get(i) {
                    if path.exists() {
                        whitelist.add_path(path.clone())?;
                        println!("    {} Added {}", "✓".green(), path.display());
                    }
                }
            }
        }
    }

    mark_setup_complete()?;

    println!();
    println!("{}", "  Setup complete!".green().bold());
    println!();
    println!("  Quick commands:");
    println!("    {} cleanser scan              {}", "→".cyan(), "Find cleanable files".dimmed());
    println!("    {} cleanser clean --dry-run   {}", "→".cyan(), "Preview what would be deleted".dimmed());
    println!("    {} cleanser clean --risk safe {}", "→".cyan(), "Clean safe items".dimmed());
    println!();

    Ok(())
}

fn get_whitelist_suggestions(home: &PathBuf, platform: platform::Platform) -> Vec<(PathBuf, &'static str)> {
    let mut suggestions = vec![
        (home.join("Documents"), "Personal documents"),
        (home.join("Desktop"), "Desktop files"),
        (home.join("Pictures"), "Photos and images"),
    ];

    match platform {
        platform::Platform::MacOS => {
            suggestions.push((home.join("Library").join("Mobile Documents"), "iCloud Drive"));
        }
        platform::Platform::Windows => {
            suggestions.push((home.join("OneDrive"), "OneDrive files"));
        }
        platform::Platform::Linux => {
            // No platform-specific suggestions for Linux
        }
    }

    suggestions
}
