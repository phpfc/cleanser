use cleanser_core::{clear_cache as core_clear_cache, get_cache_age, load_scan_results};
use colored::Colorize;
use humansize::{format_size, BINARY};

pub fn clear() -> anyhow::Result<()> {
    match core_clear_cache() {
        Ok(_) => println!("{}", "Cache cleared successfully".green()),
        Err(e) => println!("{}", format!("Failed to clear cache: {}", e).red()),
    }
    Ok(())
}

pub fn show() -> anyhow::Result<()> {
    match get_cache_age() {
        Ok(Some(age)) => {
            let mins = age / 60;
            let secs = age % 60;
            if mins > 0 {
                println!("Cache age: {} min {} sec", mins, secs);
            } else {
                println!("Cache age: {} seconds", secs);
            }

            if let Ok(Some(results)) = load_scan_results(None) {
                let total_size: u64 = results.items.iter().map(|i| i.size).sum();
                println!("Cached items: {}", results.items.len());
                println!("Total size: {}", format_size(total_size, BINARY));
            }
        }
        Ok(None) => println!("{}", "No cache found".yellow()),
        Err(e) => println!("{}", format!("Error reading cache: {}", e).red()),
    }
    Ok(())
}
