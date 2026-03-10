//! Cleanser GUI - Desktop application for disk cleanup.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod progress;

use commands::{
    add_to_whitelist, check_version, clean_items, filter_items_by_risk, get_cached_scan,
    get_current_version, get_map_stats, get_system_info, get_whitelist, rebuild_map,
    remove_from_whitelist, reveal_in_file_manager, scan,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
