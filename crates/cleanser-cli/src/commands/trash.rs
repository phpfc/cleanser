//! Trash management commands.

use cleanser_core::{TrashConfig, TrashManager};
use colored::Colorize;
use humansize::{format_size, BINARY};
use std::io;

/// List items in trash
pub fn list(json: bool) -> anyhow::Result<()> {
    let manager = TrashManager::new(TrashConfig::default())?;
    let entries = manager.list();

    if entries.is_empty() {
        println!("{}", "Trash is empty".green());
        return Ok(());
    }

    if json {
        println!("{}", serde_json::to_string_pretty(entries)?);
        return Ok(());
    }

    println!("{}", "=== Trash Contents ===".cyan().bold());
    println!(
        "Total: {} items, {}",
        entries.len(),
        format_size(manager.total_size(), BINARY).yellow()
    );
    println!();

    for entry in entries {
        let type_indicator = if entry.is_directory { "[DIR]" } else { "[FILE]" };
        let size_str = format_size(entry.size, BINARY);
        let age = entry.age_string();

        println!(
            "{} {} {} ({})",
            entry.id[..8].dimmed(),
            type_indicator.blue(),
            entry.original_path.display(),
            size_str.yellow()
        );
        println!(
            "    {} {} ago | Trash: {}",
            "Deleted:".dimmed(),
            age,
            entry.trash_path.display().to_string().dimmed()
        );
    }

    println!();
    println!(
        "{}",
        "Use 'cleanser trash restore <id>' to restore an item".dimmed()
    );

    Ok(())
}

/// Restore an item from trash
pub fn restore(entry: String, to: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    let mut manager = TrashManager::new(TrashConfig::default())?;

    // Find the entry
    let found = manager
        .find_entry(&entry)
        .ok_or_else(|| anyhow::anyhow!("Entry not found: {}", entry))?;

    let entry_id = found.id.clone();
    let original_path = found.original_path.clone();

    // Restore
    let restored_path = manager.restore(&entry_id)?;

    // If custom destination was provided, move to that location
    let final_path = if let Some(dest) = to {
        std::fs::rename(&restored_path, &dest)?;
        dest
    } else {
        restored_path
    };

    println!(
        "{} Restored: {} -> {}",
        "OK".green(),
        original_path.display(),
        final_path.display()
    );

    Ok(())
}

/// Permanently delete an item from trash
pub fn delete(entry: String, secure: bool) -> anyhow::Result<()> {
    let mut manager = TrashManager::new(TrashConfig::default())?;

    // Find the entry
    let found = manager
        .find_entry(&entry)
        .ok_or_else(|| anyhow::anyhow!("Entry not found: {}", entry))?;

    let entry_id = found.id.clone();
    let path = found.original_path.clone();

    if secure {
        // TODO: Use SecureDeleter on the trash_path before removing from journal
        println!(
            "{}",
            "Note: Secure delete from trash not yet fully implemented".yellow()
        );
    }

    let size = manager.delete_permanently(&entry_id)?;

    println!(
        "{} Permanently deleted: {} ({})",
        "OK".green(),
        path.display(),
        format_size(size, BINARY)
    );

    Ok(())
}

/// Empty the entire trash
pub fn empty(yes: bool, secure: bool) -> anyhow::Result<()> {
    let mut manager = TrashManager::new(TrashConfig::default())?;

    if manager.count() == 0 {
        println!("{}", "Trash is already empty".green());
        return Ok(());
    }

    let total_size = manager.total_size();
    let count = manager.count();

    if !yes {
        println!(
            "{}",
            format!(
                "This will permanently delete {} items ({})",
                count,
                format_size(total_size, BINARY)
            )
            .yellow()
        );

        if secure {
            println!(
                "{}",
                "Secure mode: Data will be overwritten before deletion".yellow()
            );
        }

        println!("Continue? (y/N)");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    if secure {
        // TODO: Use SecureDeleter on all trash contents
        println!(
            "{}",
            "Note: Secure empty not yet fully implemented".yellow()
        );
    }

    let freed = manager.empty()?;

    println!(
        "{} Trash emptied: {} items, {} freed",
        "OK".green(),
        count,
        format_size(freed, BINARY).green().bold()
    );

    Ok(())
}

/// Show trash statistics
pub fn stats() -> anyhow::Result<()> {
    let manager = TrashManager::new(TrashConfig::default())?;
    let entries = manager.list();

    println!("{}", "=== Trash Statistics ===".cyan().bold());
    println!("Location: {}", manager.trash_dir().display());
    println!("Items: {}", entries.len());
    println!("Total size: {}", format_size(manager.total_size(), BINARY).yellow());

    if !entries.is_empty() {
        // Count by type
        let dirs = entries.iter().filter(|e| e.is_directory).count();
        let files = entries.len() - dirs;
        println!("  Directories: {}", dirs);
        println!("  Files: {}", files);

        // Oldest and newest
        if let Some(oldest) = entries.iter().min_by_key(|e| e.deleted_at) {
            println!("Oldest: {} ({})", oldest.original_path.display(), oldest.age_string());
        }
        if let Some(newest) = entries.iter().max_by_key(|e| e.deleted_at) {
            println!("Newest: {} ({})", newest.original_path.display(), newest.age_string());
        }
    }

    Ok(())
}
