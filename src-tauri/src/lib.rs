mod api;
mod analysis;
mod commands;

use commands::{
    analyze_replay_boost, analyze_replay_rotation, fetch_replay, get_api_key, set_api_key,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            fetch_replay,
            set_api_key,
            get_api_key,
            analyze_replay_rotation,
            analyze_replay_boost,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
