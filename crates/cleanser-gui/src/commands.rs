//! Tauri commands for the GUI.

use crate::progress::TauriProgress;
use cleanser_core::{
    delete_items_with_progress, filter_by_risk, home_dir_or_err, load_scan_results,
    save_scan_results, scan_with_progress, CleanableItem, DirectoryCategory, FileSystemCrawler,
    FileSystemMap, IgnoreList, Platform, RiskLevel, ScanConfig, ScanSpeed, WhitelistConfig,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{Emitter, Window};

// ============================================================================
// DTOs (Data Transfer Objects)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ScanConfigDto {
    pub speed: String,
    pub min_file_size_mb: Option<u64>,
    pub find_duplicates: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CleanableItemDto {
    pub path: String,
    pub size: u64,
    pub category: String,
    pub risk_level: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct ScanResultsDto {
    pub items: Vec<CleanableItemDto>,
    pub total_size: u64,
    pub items_count: usize,
    pub filtered_by_size_count: usize,
    pub filtered_by_age_count: usize,
}

#[derive(Debug, Serialize)]
pub struct CleanResultDto {
    pub cleaned_count: usize,
    pub failed_count: usize,
    pub cleaned_size: u64,
    pub failures: Vec<(String, String)>,
}

#[derive(Debug, Serialize)]
pub struct SystemInfoDto {
    pub platform: String,
    pub home_dir: String,
}

// ============================================================================
// Conversion helpers
// ============================================================================

fn item_to_dto(item: &CleanableItem) -> CleanableItemDto {
    CleanableItemDto {
        path: item.path.to_string_lossy().to_string(),
        size: item.size,
        category: format!("{}", item.category),
        risk_level: match item.risk_level {
            RiskLevel::Safe => "safe".to_string(),
            RiskLevel::Moderate => "moderate".to_string(),
            RiskLevel::Risky => "risky".to_string(),
        },
        description: item.description.clone(),
    }
}

fn parse_speed(speed: &str) -> ScanSpeed {
    match speed.to_lowercase().as_str() {
        "quick" => ScanSpeed::Quick,
        "thorough" => ScanSpeed::Thorough,
        _ => ScanSpeed::Normal,
    }
}

// ============================================================================
// Commands
// ============================================================================

#[tauri::command]
pub async fn scan(window: Window, config: ScanConfigDto) -> Result<ScanResultsDto, String> {
    let home = home_dir_or_err().map_err(|e| e.to_string())?;

    // Load whitelist for ignore patterns
    let mut ignore_patterns = IgnoreList::new();
    if let Ok(whitelist) = WhitelistConfig::load() {
        for path in whitelist.list_paths() {
            let _ = ignore_patterns.add_pattern(&path.to_string_lossy());
        }
    }

    let scan_config = ScanConfig {
        speed: parse_speed(&config.speed),
        paths: vec![home],
        min_file_size_mb: config.min_file_size_mb.unwrap_or(100),
        max_depth: Some(6),
        find_duplicates: config.find_duplicates.unwrap_or(false),
        ignore_patterns,
        size_range: None,
        age_criteria: None,
    };

    let progress = TauriProgress::new(window);
    let results = scan_with_progress(scan_config, &progress).map_err(|e| e.to_string())?;

    // Save to cache
    let _ = save_scan_results(&results);

    let dto = ScanResultsDto {
        items: results.items.iter().map(item_to_dto).collect(),
        total_size: results.total_size,
        items_count: results.items.len(),
        filtered_by_size_count: results.filtered_by_size_count,
        filtered_by_age_count: results.filtered_by_age_count,
    };

    Ok(dto)
}

#[tauri::command]
pub async fn clean_items(
    window: Window,
    paths: Vec<String>,
    dry_run: bool,
) -> Result<CleanResultDto, String> {
    // Load cached scan results to get full item info
    let cached = load_scan_results(None)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No cached scan results found. Please run a scan first.".to_string())?;

    // Find items matching the provided paths
    let path_set: std::collections::HashSet<PathBuf> =
        paths.iter().map(PathBuf::from).collect();

    let items_to_delete: Vec<CleanableItem> = cached
        .items
        .into_iter()
        .filter(|item| path_set.contains(&item.path))
        .collect();

    if items_to_delete.is_empty() {
        return Ok(CleanResultDto {
            cleaned_count: 0,
            failed_count: 0,
            cleaned_size: 0,
            failures: vec![],
        });
    }

    let progress = TauriProgress::new(window);
    let result =
        delete_items_with_progress(&items_to_delete, dry_run, &progress).map_err(|e| e.to_string())?;

    Ok(CleanResultDto {
        cleaned_count: result.cleaned_count,
        failed_count: result.failed_count,
        cleaned_size: result.cleaned_size,
        failures: result
            .failures
            .into_iter()
            .map(|(p, e)| (p.to_string_lossy().to_string(), e))
            .collect(),
    })
}

#[tauri::command]
pub fn get_cached_scan() -> Result<Option<ScanResultsDto>, String> {
    match load_scan_results(Some(3600)) {
        Ok(Some(results)) => {
            let dto = ScanResultsDto {
                items: results.items.iter().map(item_to_dto).collect(),
                total_size: results.total_size,
                items_count: results.items.len(),
                filtered_by_size_count: results.filtered_by_size_count,
                filtered_by_age_count: results.filtered_by_age_count,
            };
            Ok(Some(dto))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn filter_items_by_risk(risk: String) -> Result<Vec<CleanableItemDto>, String> {
    let cached = load_scan_results(None)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No cached scan results".to_string())?;

    let risk_level = match risk.to_lowercase().as_str() {
        "safe" => RiskLevel::Safe,
        "moderate" => RiskLevel::Moderate,
        _ => RiskLevel::Risky,
    };

    let filtered = filter_by_risk(&cached, risk_level);
    Ok(filtered.iter().map(item_to_dto).collect())
}

#[tauri::command]
pub fn get_whitelist() -> Result<Vec<String>, String> {
    let whitelist = WhitelistConfig::load().map_err(|e| e.to_string())?;
    Ok(whitelist
        .list_paths()
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

#[tauri::command]
pub fn add_to_whitelist(path: String) -> Result<(), String> {
    let mut whitelist = WhitelistConfig::load().unwrap_or_else(|_| WhitelistConfig::new());
    whitelist
        .add_path(PathBuf::from(path))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn remove_from_whitelist(path: String) -> Result<bool, String> {
    let mut whitelist = WhitelistConfig::load().map_err(|e| e.to_string())?;
    whitelist
        .remove_path(&PathBuf::from(path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_system_info() -> SystemInfoDto {
    let platform = Platform::current();
    let home = home_dir_or_err()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "Unknown".to_string());

    SystemInfoDto {
        platform: platform.name().to_string(),
        home_dir: home,
    }
}

// ============================================================================
// File Operations Commands
// ============================================================================

#[tauri::command]
pub async fn reveal_in_file_manager(path: String) -> Result<(), String> {
    let path = PathBuf::from(&path);

    #[cfg(target_os = "macos")]
    {
        // On macOS, use 'open -R' to reveal and select the file in Finder
        std::process::Command::new("open")
            .args(["-R", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Failed to reveal in Finder: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, use explorer /select to reveal and select the file
        std::process::Command::new("explorer")
            .args(["/select,", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Failed to reveal in Explorer: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux, try xdg-open on the parent directory
        // Note: Different file managers handle this differently
        let target_path = if path.is_file() {
            path.parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| path.clone())
        } else {
            path.clone()
        };
        std::process::Command::new("xdg-open")
            .arg(&target_path.to_string_lossy().to_string())
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }

    Ok(())
}

// ============================================================================
// Map Commands
// ============================================================================

#[derive(Debug, Serialize)]
pub struct MapStatsDto {
    pub total_directories: usize,
    pub cleanable_count: usize,
    pub created_at: String,
    pub is_stale: bool,
    pub categories: HashMap<String, usize>,
    pub tags: Vec<(String, usize)>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MapProgressDto {
    pub message: String,
    pub current: usize,
    pub total: usize,
}

/// Progress callback for map rebuild
struct TauriCrawlerProgress {
    window: std::sync::Arc<Window>,
}

impl cleanser_core::CrawlerProgress for TauriCrawlerProgress {
    fn on_start(&self, message: &str) {
        let _ = self.window.emit(
            "map:progress",
            MapProgressDto {
                message: message.to_string(),
                current: 0,
                total: 0,
            },
        );
    }

    fn on_progress(&self, current: usize, total: usize, path: &std::path::Path) {
        let _ = self.window.emit(
            "map:progress",
            MapProgressDto {
                message: path.to_string_lossy().to_string(),
                current,
                total,
            },
        );
    }

    fn on_complete(&self, message: &str) {
        let _ = self.window.emit(
            "map:progress",
            MapProgressDto {
                message: message.to_string(),
                current: 100,
                total: 100,
            },
        );
    }
}

#[tauri::command]
pub fn get_map_stats() -> Result<Option<MapStatsDto>, String> {
    match FileSystemMap::load() {
        Ok(map) => {
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

            let created = chrono::DateTime::from_timestamp(map.created_at as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let mut categories: HashMap<String, usize> = HashMap::new();
            for dir in map.directories.values() {
                let cat_name = match dir.category {
                    DirectoryCategory::Ephemeral => "Cache/Temp",
                    DirectoryCategory::BuildArtifact => "Build Artifacts",
                    DirectoryCategory::ApplicationData => "App Data",
                    DirectoryCategory::UserContent => "User Content",
                    DirectoryCategory::System => "System",
                    DirectoryCategory::Unknown => "Other",
                };
                *categories.entry(cat_name.to_string()).or_insert(0) += 1;
            }

            let tag_stats = map.stats_by_tag();
            let mut tags: Vec<(String, usize)> = tag_stats
                .into_iter()
                .map(|(tag, (count, _))| (tag, count))
                .collect();
            tags.sort_by(|a, b| b.1.cmp(&a.1));
            tags.truncate(10);

            Ok(Some(MapStatsDto {
                total_directories: map.total_directories,
                cleanable_count,
                created_at: created,
                is_stale: map.is_stale(),
                categories,
                tags,
            }))
        }
        Err(_) => Ok(None),
    }
}

#[tauri::command]
pub async fn rebuild_map(window: Window) -> Result<MapStatsDto, String> {
    let progress = Box::new(TauriCrawlerProgress {
        window: std::sync::Arc::new(window),
    });

    let crawler = FileSystemCrawler::new()
        .with_max_depth(6)
        .with_min_confidence(0.7)
        .with_progress_callback(progress);

    let map = crawler.crawl_full().map_err(|e| e.to_string())?;
    map.save().map_err(|e| e.to_string())?;

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

    let created = chrono::DateTime::from_timestamp(map.created_at as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let mut categories: HashMap<String, usize> = HashMap::new();
    for dir in map.directories.values() {
        let cat_name = match dir.category {
            DirectoryCategory::Ephemeral => "Cache/Temp",
            DirectoryCategory::BuildArtifact => "Build Artifacts",
            DirectoryCategory::ApplicationData => "App Data",
            DirectoryCategory::UserContent => "User Content",
            DirectoryCategory::System => "System",
            DirectoryCategory::Unknown => "Other",
        };
        *categories.entry(cat_name.to_string()).or_insert(0) += 1;
    }

    let tag_stats = map.stats_by_tag();
    let mut tags: Vec<(String, usize)> = tag_stats
        .into_iter()
        .map(|(tag, (count, _))| (tag, count))
        .collect();
    tags.sort_by(|a, b| b.1.cmp(&a.1));
    tags.truncate(10);

    Ok(MapStatsDto {
        total_directories: map.total_directories,
        cleanable_count,
        created_at: created,
        is_stale: false,
        categories,
        tags,
    })
}
