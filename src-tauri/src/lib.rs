mod api;
mod analysis;
mod commands;
mod parser;

use std::collections::HashMap;
use std::sync::Mutex;

use commands::{
    analyze_replay_boost, analyze_replay_rotation, download_and_parse_replay, fetch_replay,
    get_api_key, set_api_key, ReplayCache,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(ReplayCache(Mutex::new(HashMap::new())))
        .invoke_handler(tauri::generate_handler![
            fetch_replay,
            set_api_key,
            get_api_key,
            analyze_replay_rotation,
            analyze_replay_boost,
            download_and_parse_replay,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
