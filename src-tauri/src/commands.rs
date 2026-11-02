use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;
use tauri_plugin_store::StoreExt;

use crate::api::ballchasing::BallchasingClient;
use crate::api::models::{PlayerData, ReplayData};
use crate::analysis::boost::{analyze_boost, BoostInsight};
use crate::analysis::rotation::{analyze_rotation, RotationInsight};
use crate::parser::frame_data::FrameData;
use crate::parser::replay_parser;

pub struct ReplayCache(pub Mutex<HashMap<String, ReplayData>>);

#[tauri::command]
pub async fn fetch_replay(
    app: tauri::AppHandle,
    cache: State<'_, ReplayCache>,
    replay_id: String,
) -> Result<ReplayData, String> {
    if let Ok(map) = cache.0.lock()
        && let Some(cached) = map.get(&replay_id) {
            return Ok(cached.clone());
    }

    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    let api_key = store
        .get("api_key")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .ok_or("No API key configured. Please set your ballchasing.com API key.")?;

    let client = BallchasingClient::new(api_key);
    let replay = client.fetch_replay(&replay_id).await?;

    // Debug: log first player's demo stats
    if let Some(blue) = &replay.blue
        && let Some(players) = &blue.players
        && let Some(p) = players.first() {
            eprintln!("Player '{}' demo stats: {:?}", p.name.as_deref().unwrap_or("?"), p.stats.as_ref().and_then(|s| s.demo.as_ref()));
    }

    if let Ok(mut map) = cache.0.lock() {
        map.insert(replay_id, replay.clone());
    }

    Ok(replay)
}

#[tauri::command]
pub async fn download_and_parse_replay(
    app: tauri::AppHandle,
    replay_id: String,
) -> Result<FrameData, String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    let api_key = store
        .get("api_key")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .ok_or("No API key configured.")?;

    let client = BallchasingClient::new(api_key);
    let replay_bytes = client.download_replay(&replay_id).await
        .map_err(|e| format!("Download failed: {}", e))?;

    eprintln!("Downloaded replay: {} bytes", replay_bytes.len());

    let result = tokio::task::spawn_blocking(move || {
        replay_parser::parse_replay(&replay_bytes)
            .map_err(|e| format!("Parse failed: {}", e))
    })
    .await
    .map_err(|e| format!("Parse task panicked: {}", e))?;

    match &result {
        Ok(data) => eprintln!("Parse OK: {} frames, {} players", data.total_frames, data.players.len()),
        Err(e) => eprintln!("Parse error: {}", e),
    }
    result
}

#[tauri::command]
pub async fn set_api_key(app: tauri::AppHandle, key: String) -> Result<(), String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set("api_key", serde_json::json!(key));
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_api_key(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    let key = store
        .get("api_key")
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    Ok(key)
}

#[tauri::command]
pub fn analyze_replay_rotation(players: Vec<PlayerData>) -> Vec<RotationInsight> {
    analyze_rotation(&players)
}

#[tauri::command]
pub fn analyze_replay_boost(players: Vec<PlayerData>) -> Vec<BoostInsight> {
    analyze_boost(&players)
}
