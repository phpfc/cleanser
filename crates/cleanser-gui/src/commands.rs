//! Tauri commands for the GUI.

use crate::progress::TauriProgress;
use cleanser_core::{
    check_for_updates, delete_items_with_progress, filter_by_risk, home_dir_or_err,
    load_scan_results, save_scan_results, scan_with_progress, CleanableItem, DirectoryCategory,
    FileSystemCrawler, FileSystemMap, IgnoreList, Platform, RiskLevel, ScanConfig, ScanSpeed,
    ScheduleFrequency, ScheduledJob, Scheduler, TrashConfig, TrashManager, VersionInfo,
    WhitelistConfig,
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
    let path_set: std::collections::HashSet<PathBuf> = paths.iter().map(PathBuf::from).collect();

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
    let result = delete_items_with_progress(&items_to_delete, dry_run, &progress)
        .map_err(|e| e.to_string())?;

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

// ============================================================================
// Version Commands
// ============================================================================

#[derive(Debug, Serialize)]
pub struct VersionInfoDto {
    pub current: String,
    pub latest: Option<String>,
    pub update_available: bool,
    pub release_url: Option<String>,
}

impl From<VersionInfo> for VersionInfoDto {
    fn from(info: VersionInfo) -> Self {
        Self {
            current: info.current,
            latest: info.latest,
            update_available: info.update_available,
            release_url: info.release_url,
        }
    }
}

#[tauri::command]
pub async fn check_version() -> Result<VersionInfoDto, String> {
    check_for_updates()
        .await
        .map(|info| info.into())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_current_version() -> String {
    cleanser_core::current_version().to_string()
}

// ============================================================================
// Trash Commands
// ============================================================================

#[derive(Debug, Serialize)]
pub struct TrashEntryDto {
    pub id: String,
    pub original_path: String,
    pub trash_path: String,
    pub size: u64,
    pub is_directory: bool,
    pub deleted_at: String,
    pub age: String,
}

#[derive(Debug, Serialize)]
pub struct TrashStatsDto {
    pub location: String,
    pub item_count: usize,
    pub total_size: u64,
    pub directories: usize,
    pub files: usize,
}

#[tauri::command]
pub fn get_trash_items() -> Result<Vec<TrashEntryDto>, String> {
    let manager = TrashManager::new(TrashConfig::default()).map_err(|e| e.to_string())?;
    let entries = manager.list();

    Ok(entries
        .iter()
        .map(|e| TrashEntryDto {
            id: e.id.clone(),
            original_path: e.original_path.to_string_lossy().to_string(),
            trash_path: e.trash_path.to_string_lossy().to_string(),
            size: e.size,
            is_directory: e.is_directory,
            deleted_at: e.deleted_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            age: e.age_string(),
        })
        .collect())
}

#[tauri::command]
pub fn get_trash_stats() -> Result<TrashStatsDto, String> {
    let manager = TrashManager::new(TrashConfig::default()).map_err(|e| e.to_string())?;
    let entries = manager.list();

    let directories = entries.iter().filter(|e| e.is_directory).count();
    let files = entries.len() - directories;

    Ok(TrashStatsDto {
        location: manager.trash_dir().to_string_lossy().to_string(),
        item_count: entries.len(),
        total_size: manager.total_size(),
        directories,
        files,
    })
}

#[tauri::command]
pub fn restore_trash_item(entry_id: String, to_path: Option<String>) -> Result<String, String> {
    let mut manager = TrashManager::new(TrashConfig::default()).map_err(|e| e.to_string())?;

    let restored_path = manager.restore(&entry_id).map_err(|e| e.to_string())?;

    // If custom destination was provided, move to that location
    let final_path = if let Some(dest) = to_path {
        let dest_path = std::path::PathBuf::from(&dest);
        std::fs::rename(&restored_path, &dest_path).map_err(|e| e.to_string())?;
        dest_path
    } else {
        restored_path
    };

    Ok(final_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn delete_trash_item(entry_id: String) -> Result<u64, String> {
    let mut manager = TrashManager::new(TrashConfig::default()).map_err(|e| e.to_string())?;
    manager
        .delete_permanently(&entry_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn empty_trash() -> Result<u64, String> {
    let mut manager = TrashManager::new(TrashConfig::default()).map_err(|e| e.to_string())?;
    manager.empty().map_err(|e| e.to_string())
}

// ============================================================================
// Schedule Commands
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ScheduledJobDto {
    pub id: String,
    pub name: String,
    pub frequency: String,
    pub risk_level: String,
    pub enabled: bool,
    pub use_trash: bool,
    pub secure_delete: bool,
    pub last_run: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateJobDto {
    pub name: String,
    pub frequency: String,
    pub risk_level: String,
    pub use_trash: bool,
    pub secure_delete: bool,
    pub notify: bool,
}

#[tauri::command]
pub fn get_scheduled_jobs() -> Result<Vec<ScheduledJobDto>, String> {
    let scheduler = Scheduler::new().map_err(|e| e.to_string())?;
    let jobs = scheduler.list_jobs();

    Ok(jobs
        .iter()
        .map(|j| ScheduledJobDto {
            id: j.id.clone(),
            name: j.name.clone(),
            frequency: j.frequency.description(),
            risk_level: format!("{}", j.risk_level),
            enabled: j.enabled,
            use_trash: j.use_trash,
            secure_delete: j.secure_delete,
            last_run: j
                .last_run
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
        })
        .collect())
}

#[tauri::command]
pub fn create_scheduled_job(job: CreateJobDto) -> Result<ScheduledJobDto, String> {
    let freq = ScheduleFrequency::parse(&job.frequency).map_err(|e| e.to_string())?;

    let risk_level = match job.risk_level.to_lowercase().as_str() {
        "safe" => RiskLevel::Safe,
        "moderate" => RiskLevel::Moderate,
        "risky" => RiskLevel::Risky,
        _ => RiskLevel::Safe,
    };

    let mut scheduled_job = ScheduledJob::new(job.name.clone(), freq);
    scheduled_job.risk_level = risk_level;
    scheduled_job.use_trash = job.use_trash;
    scheduled_job.secure_delete = job.secure_delete;
    scheduled_job.notify_on_complete = job.notify;

    let mut scheduler = Scheduler::new().map_err(|e| e.to_string())?;
    scheduler
        .create_job(scheduled_job.clone())
        .map_err(|e| e.to_string())?;

    Ok(ScheduledJobDto {
        id: scheduled_job.id,
        name: scheduled_job.name,
        frequency: scheduled_job.frequency.description(),
        risk_level: format!("{}", scheduled_job.risk_level),
        enabled: scheduled_job.enabled,
        use_trash: scheduled_job.use_trash,
        secure_delete: scheduled_job.secure_delete,
        last_run: None,
    })
}

#[tauri::command]
pub fn remove_scheduled_job(job_name: String) -> Result<(), String> {
    let mut scheduler = Scheduler::new().map_err(|e| e.to_string())?;
    scheduler.remove_job(&job_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn enable_scheduled_job(job_name: String) -> Result<(), String> {
    let mut scheduler = Scheduler::new().map_err(|e| e.to_string())?;
    scheduler.enable_job(&job_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn disable_scheduled_job(job_name: String) -> Result<(), String> {
    let mut scheduler = Scheduler::new().map_err(|e| e.to_string())?;
    scheduler.disable_job(&job_name).map_err(|e| e.to_string())
}
