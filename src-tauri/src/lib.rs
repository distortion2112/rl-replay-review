mod api;
mod analysis;
mod commands;
pub mod parser;

use std::collections::HashMap;
use std::sync::Mutex;

use api::rate_limit::RateLimiter;
use commands::{
    analyze_replay_boost, analyze_replay_rotation, download_and_parse_replay, fetch_replay,
    get_api_key, get_rate_limit_status, search_player_replays, set_api_key, ReplayCache,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(RateLimiter::new())
        .manage(ReplayCache(Mutex::new(HashMap::new())))
        .invoke_handler(tauri::generate_handler![
            fetch_replay,
            set_api_key,
            get_api_key,
            get_rate_limit_status,
            search_player_replays,
            analyze_replay_rotation,
            analyze_replay_boost,
            download_and_parse_replay,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
