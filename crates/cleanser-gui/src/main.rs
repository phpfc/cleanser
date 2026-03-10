//! Cleanser GUI - Desktop application for disk cleanup.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod progress;

use commands::{
    add_to_whitelist, check_version, clean_items, create_scheduled_job, delete_trash_item,
    disable_scheduled_job, empty_trash, enable_scheduled_job, filter_items_by_risk,
    get_cached_scan, get_current_version, get_map_stats, get_scheduled_jobs, get_system_info,
    get_trash_items, get_trash_stats, get_whitelist, rebuild_map, remove_from_whitelist,
    remove_scheduled_job, restore_trash_item, reveal_in_file_manager, scan,
};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            scan,
            clean_items,
            get_cached_scan,
            filter_items_by_risk,
            get_whitelist,
            add_to_whitelist,
            remove_from_whitelist,
            get_system_info,
            get_map_stats,
            rebuild_map,
            reveal_in_file_manager,
            check_version,
            get_current_version,
            // Trash commands
            get_trash_items,
            get_trash_stats,
            restore_trash_item,
            delete_trash_item,
            empty_trash,
            // Schedule commands
            get_scheduled_jobs,
            create_scheduled_job,
            remove_scheduled_job,
            enable_scheduled_job,
            disable_scheduled_job,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
