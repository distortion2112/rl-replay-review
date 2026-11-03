use std::collections::HashMap;
use std::sync::Mutex;
use tauri::path::BaseDirectory;
use tauri::{Emitter, Manager, State};
use tauri_plugin_store::StoreExt;

use crate::api::ballchasing::BallchasingClient;
use crate::benchmark::{self, Benchmarks, RankLookup};
use crate::coaching::{self, CoachingReport, EvidencePack};
use crate::profile::{self, PlayerAggregate, PlaystyleProfile, Skipped};
use crate::api::models::{PlayerData, ReplayData, ReplayListResponse, ReplaySearchParams};
use crate::api::rate_limit::{RateLimitStatus, RateLimiter};
use crate::analysis::boost::{analyze_boost, BoostInsight};
use crate::analysis::rotation::{analyze_rotation, RotationInsight};
use crate::parser::frame_data::FrameData;
use crate::parser::replay_parser;

pub struct ReplayCache(pub Mutex<HashMap<String, ReplayData>>);

#[tauri::command]
pub async fn fetch_replay(
    app: tauri::AppHandle,
    cache: State<'_, ReplayCache>,
    rate_limiter: State<'_, RateLimiter>,
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

    let client = BallchasingClient::new(api_key, (*rate_limiter).clone());
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
    rate_limiter: State<'_, RateLimiter>,
    replay_id: String,
) -> Result<FrameData, String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    let api_key = store
        .get("api_key")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .ok_or("No API key configured.")?;

    let client = BallchasingClient::new(api_key, (*rate_limiter).clone());
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
pub async fn set_api_key(
    app: tauri::AppHandle,
    rate_limiter: State<'_, RateLimiter>,
    key: String,
) -> Result<(), String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set("api_key", serde_json::json!(key));
    store.save().map_err(|e| e.to_string())?;
    // A different key can belong to a different patron tier
    rate_limiter.reset_tier().await;
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
pub async fn get_rate_limit_status(
    rate_limiter: State<'_, RateLimiter>,
) -> Result<RateLimitStatus, String> {
    Ok(rate_limiter.status().await)
}

#[tauri::command]
pub async fn search_player_replays(
    app: tauri::AppHandle,
    rate_limiter: State<'_, RateLimiter>,
    params: ReplaySearchParams,
) -> Result<ReplayListResponse, String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    let api_key = store
        .get("api_key")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .ok_or("No API key configured.")?;

    let client = BallchasingClient::new(api_key, (*rate_limiter).clone());
    client.search_replays(&params).await
}

#[tauri::command]
pub fn analyze_replay_rotation(players: Vec<PlayerData>) -> Vec<RotationInsight> {
    analyze_rotation(&players)
}

#[tauri::command]
pub fn analyze_replay_boost(players: Vec<PlayerData>) -> Vec<BoostInsight> {
    analyze_boost(&players)
}

/// Parsed `benchmarks.json`, loaded on first use.
#[derive(Default)]
pub struct BenchmarkCache(pub Mutex<Option<std::sync::Arc<Benchmarks>>>);

fn load_benchmarks(app: &tauri::AppHandle, cache: &BenchmarkCache) -> Result<std::sync::Arc<Benchmarks>, String> {
    if let Ok(guard) = cache.0.lock()
        && let Some(data) = guard.as_ref()
    {
        return Ok(data.clone());
    }
    let path = app
        .path()
        .resolve("resources/benchmarks.json", BaseDirectory::Resource)
        .map_err(|e| format!("Benchmarks file not found: {}", e))?;
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Couldn't read benchmarks file {}: {}", path.display(), e))?;
    let data = std::sync::Arc::new(benchmark::parse(&json)?);
    if let Ok(mut guard) = cache.0.lock() {
        *guard = Some(data.clone());
    }
    Ok(data)
}

/// Benchmark for a ballchasing rank id (e.g. `diamond-2`), with the nearest tiers
/// below and above. Falls back to the nearest tier with data and says so.
#[tauri::command]
pub fn get_rank_benchmark(
    app: tauri::AppHandle,
    cache: State<'_, BenchmarkCache>,
    rank: String,
) -> Result<RankLookup, String> {
    let data = load_benchmarks(&app, &cache)?;
    benchmark::lookup(&data, &rank)
}

/// Raw replay details by id, shared by aggregate builds within a session.
#[derive(Default)]
pub struct RawReplayCache(pub Mutex<HashMap<String, serde_json::Value>>);

#[derive(Clone, serde::Serialize)]
struct AggregateProgress {
    current: usize,
    total: usize,
    replay_id: String,
}

/// Builds a player's per-game stats across `replay_ids`, cache-first.
/// Emits `aggregate-progress` as each replay is processed. Replays that fail
/// to fetch or don't contain a full game for the player end up in `skipped`.
#[tauri::command]
pub async fn build_player_aggregate(
    app: tauri::AppHandle,
    rate_limiter: State<'_, RateLimiter>,
    cache: State<'_, RawReplayCache>,
    player_id: String,
    replay_ids: Vec<String>,
) -> Result<PlayerAggregate, String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    let api_key = store
        .get("api_key")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .ok_or("No API key configured.")?;
    let client = BallchasingClient::new(api_key, (*rate_limiter).clone());

    let total = replay_ids.len();
    let mut details = Vec::with_capacity(total);
    let mut fetch_errors = Vec::new();
    for (i, id) in replay_ids.iter().enumerate() {
        let cached = cache.0.lock().ok().and_then(|m| m.get(id).cloned());
        let detail = match cached {
            Some(d) => Some(d),
            None => match client.fetch_replay_raw(id).await {
                Ok(d) => {
                    if let Ok(mut m) = cache.0.lock() {
                        m.insert(id.clone(), d.clone());
                    }
                    Some(d)
                }
                Err(e) => {
                    fetch_errors.push(Skipped { replay_id: id.clone(), reason: e });
                    None
                }
            },
        };
        details.extend(detail);
        let _ = app.emit("aggregate-progress", AggregateProgress { current: i + 1, total, replay_id: id.clone() });
    }
    let mut aggregate = PlayerAggregate::from_details(&player_id, details.iter());
    aggregate.skipped.extend(fetch_errors);
    Ok(aggregate)
}

/// Evidence pack comparing `aggregate` to the benchmarks at `rank` and the tier
/// above it. Fewer than 10 games returns a pack with `enough_data: false`.
#[tauri::command]
pub fn generate_coaching_report(
    app: tauri::AppHandle,
    cache: State<'_, BenchmarkCache>,
    aggregate: PlayerAggregate,
    rank: String,
) -> Result<EvidencePack, String> {
    let data = load_benchmarks(&app, &cache)?;
    coaching::generate_report(&aggregate, &rank, &data)
}

/// How a player plays on five axes, relative to their rank. Descriptive only:
/// coaching never treats an axis as a weakness.
#[tauri::command]
pub fn compute_playstyle(
    app: tauri::AppHandle,
    cache: State<'_, BenchmarkCache>,
    aggregate: PlayerAggregate,
    rank: String,
) -> Result<PlaystyleProfile, String> {
    let data = load_benchmarks(&app, &cache)?;
    profile::compute_playstyle(&aggregate, &rank, &data)
}

/// Readable coaching from an evidence pack, using deterministic templates.
/// Every number shown comes from the pack.
#[tauri::command]
pub fn render_coaching_report(pack: EvidencePack) -> CoachingReport {
    coaching::render(&pack)
}
