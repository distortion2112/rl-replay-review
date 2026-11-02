use std::collections::HashMap;

use boxcars::{Attribute, HeaderProp, ParserBuilder};

use super::frame_data::*;

/// Longest staleness a sampled position is projected forward by its velocity.
/// Normal update gaps are <=~0.1s; anything older is left as-is.
const MAX_EXTRAPOLATION_S: f32 = 0.25;

pub fn parse_replay(data: &[u8]) -> Result<FrameData, String> {
    let replay = ParserBuilder::new(data)
        .on_error_check_crc()
        .must_parse_network_data()
        .parse()
        .or_else(|_| {
            eprintln!("Full network parse failed, retrying with error tolerance...");
            ParserBuilder::new(data)
                .on_error_check_crc()
                .ignore_network_data_on_error()
                .parse()
        })
        .map_err(|e| format!("Failed to parse replay: {:?}", e))?;

    let network_frames = replay
        .network_frames
        .as_ref()
        .ok_or("No network data could be extracted from this replay.")?;

    let objects = &replay.objects;

    // Exact archetype IDs
    let car_archetype = find_object_id(objects, "Archetypes.Car.Car_Default");
    let ball_archetype = find_object_id(objects, "Archetypes.Ball.Ball_Default");
    let boost_archetype = find_object_id(objects, "Archetypes.CarComponents.CarComponent_Boost");

    // Attribute object IDs
    let player_name_attr = find_object_id(objects, "Engine.PlayerReplicationInfo:PlayerName");
    let vehicle_attr = find_object_id(objects, "TAGame.CarComponent_TA:Vehicle");

    eprintln!("Archetypes: car={:?} ball={:?} boost={:?}", car_archetype, ball_archetype, boost_archetype);
    eprintln!("Attrs: name={:?} vehicle={:?}", player_name_attr, vehicle_attr);

    // Extract player info from header
    let mut header_players: Vec<FramePlayer> = Vec::new();
    if let Some(HeaderProp::Array(player_stats)) =
        replay.properties.iter().find(|(k, _)| k == "PlayerStats").map(|(_, v)| v)
    {
        for (idx, player) in player_stats.iter().enumerate() {
            let name = player.iter()
                .find(|(k, _)| k == "Name")
                .and_then(|(_, v)| if let HeaderProp::Str(s) = v { Some(s.clone()) } else { None })
                .unwrap_or_else(|| format!("Player {}", idx));
            let team = player.iter()
                .find(|(k, _)| k == "Team")
                .and_then(|(_, v)| if let HeaderProp::Int(t) = v { Some(*t as u8) } else { None })
                .unwrap_or(0);
            header_players.push(FramePlayer { index: idx, name, team });
        }
    }
    eprintln!("Header players: {:?}", header_players.iter().map(|p| format!("{}(t{})", p.name, p.team)).collect::<Vec<_>>());

    // Actor IDs are recycled: when a car is destroyed (goal reset, demo) its ID can be
    // reassigned to a different player's next car. All car <-> player linkage is therefore
    // tracked live, per frame, and dropped when the actor is deleted. A whole-replay
    // car -> player map attributes positions to the wrong players.
    let pawn_pri_attr = find_object_id(objects, "Engine.Pawn:PlayerReplicationInfo");

    let mut goals: Vec<GoalEvent> = Vec::new();
    for tick in &replay.tick_marks {
        if tick.description.contains("Goal") {
            goals.push(GoalEvent { frame: tick.frame as usize, time: 0.0 });
        }
    }

    let mut game_frames: Vec<GameFrame> = Vec::new();
    let mut actor_positions: HashMap<i32, Position> = HashMap::new();
    // Each rigid body updates on its own irregular schedule (every 1-3 network frames),
    // so at a sample frame a position can be up to ~70ms stale by a varying amount.
    // Keep when it was last updated and at what velocity, so samples can be projected
    // to the sample time; otherwise per-sample movement is uneven and playback pulses.
    let mut actor_motion: HashMap<i32, (f32, boxcars::Vector3f)> = HashMap::new();
    let mut ball_actor_id: Option<i32> = None;
    let mut alive_cars: Vec<i32> = Vec::new();
    let mut live_car_to_pri: HashMap<i32, i32> = HashMap::new();
    let mut live_pri_name: HashMap<i32, String> = HashMap::new();
    let mut live_boost_amount: HashMap<i32, u8> = HashMap::new();
    let mut live_boost_to_car: HashMap<i32, i32> = HashMap::new();

    for (frame_idx, frame) in network_frames.frames.iter().enumerate() {
        // Deletions are processed before new actors so a recycled ID starts clean.
        for deleted in &frame.deleted_actors {
            let id = deleted.0;
            actor_positions.remove(&id);
            actor_motion.remove(&id);
            alive_cars.retain(|c| *c != id);
            live_car_to_pri.remove(&id);
            live_pri_name.remove(&id);
            live_boost_to_car.remove(&id);
            live_boost_amount.remove(&id);
            if ball_actor_id == Some(id) {
                ball_actor_id = None;
            }
        }

        for na in &frame.new_actors {
            let aid = na.actor_id.0;
            let oid = na.object_id.0;
            if Some(oid) == ball_archetype {
                ball_actor_id = Some(aid);
            }
            if Some(oid) == car_archetype && !alive_cars.contains(&aid) {
                alive_cars.push(aid);
            }
            if let Some(loc) = &na.initial_trajectory.location {
                actor_positions.insert(aid, Position {
                    x: loc.x as f32,
                    y: loc.y as f32,
                    z: loc.z as f32,
                });
                actor_motion.remove(&aid);
            }
        }

        for update in &frame.updated_actors {
            let aid = update.actor_id.0;
            let oid = update.object_id.0;
            match &update.attribute {
                Attribute::RigidBody(rb) => {
                    actor_positions.insert(aid, Position {
                        x: rb.location.x,
                        y: rb.location.y,
                        z: rb.location.z,
                    });
                    match rb.linear_velocity {
                        Some(v) if !rb.sleeping => {
                            actor_motion.insert(aid, (frame.time, v));
                        }
                        _ => {
                            actor_motion.remove(&aid);
                        }
                    }
                }
                Attribute::String(name) if Some(oid) == player_name_attr => {
                    live_pri_name.insert(aid, name.clone());
                }
                Attribute::ReplicatedBoost(boost) => {
                    live_boost_amount.insert(aid, boost.boost_amount);
                }
                Attribute::ActiveActor(aa) if Some(oid) == vehicle_attr => {
                    if aa.active {
                        live_boost_to_car.insert(aid, aa.actor.0);
                    }
                }
                Attribute::ActiveActor(aa) if Some(oid) == pawn_pri_attr => {
                    if aa.active {
                        live_car_to_pri.insert(aid, aa.actor.0);
                    } else {
                        live_car_to_pri.remove(&aid);
                    }
                }
                Attribute::ActiveActor(aa) if pawn_pri_attr.is_none() && aa.active => {
                    // Fallback for replays without the Pawn PRI attribute: sniff any
                    // ActiveActor that links a live car to a known PRI, or vice versa.
                    let target = aa.actor.0;
                    if alive_cars.contains(&aid) && live_pri_name.contains_key(&target) {
                        live_car_to_pri.insert(aid, target);
                    } else if live_pri_name.contains_key(&aid) && alive_cars.contains(&target) {
                        live_car_to_pri.insert(target, aid);
                    }
                }
                _ => {}
            }
        }

        // Sample every 3 frames
        if frame_idx % 3 == 0 {
            let position_at = |id: i32| -> Option<Position> {
                let pos = actor_positions.get(&id).copied()?;
                Some(match actor_motion.get(&id) {
                    Some((updated, v)) => {
                        let age = frame.time - updated;
                        if age > 0.0 && age <= MAX_EXTRAPOLATION_S {
                            Position { x: pos.x + v.x * age, y: pos.y + v.y * age, z: pos.z + v.z * age }
                        } else {
                            pos
                        }
                    }
                    None => pos,
                })
            };
            let ball_pos = ball_actor_id.and_then(position_at);

            let mut car_boost: HashMap<i32, u8> = HashMap::new();
            for (comp, car) in &live_boost_to_car {
                if let Some(amount) = live_boost_amount.get(comp) {
                    car_boost.insert(*car, *amount);
                }
            }

            let mut player_frames = Vec::new();
            let mut seen_players: Vec<usize> = Vec::new();
            for &car_id in &alive_cars {
                let player_index = live_car_to_pri
                    .get(&car_id)
                    .and_then(|pri| live_pri_name.get(pri))
                    .and_then(|name| header_players.iter().find(|p| p.name == *name))
                    .map(|p| p.index);
                let Some(player_index) = player_index else { continue };
                if seen_players.contains(&player_index) {
                    continue;
                }
                seen_players.push(player_index);

                player_frames.push(PlayerFrame {
                    player_index,
                    position: position_at(car_id),
                    boost_amount: car_boost.get(&car_id).copied(),
                });
            }

            game_frames.push(GameFrame {
                time: frame.time,
                ball: ball_pos,
                players: player_frames,
            });
        }
    }

    // Fill goal times
    for goal in &mut goals {
        if goal.frame < network_frames.frames.len() {
            goal.time = network_frames.frames[goal.frame].time;
        }
    }

    // Debug mid-frame
    if let Some(mid) = game_frames.get(game_frames.len() / 2) {
        eprintln!("Mid-frame t={:.1}s: ball={:?}, {} players", mid.time, mid.ball, mid.players.len());
        for pf in &mid.players {
            let name = header_players.iter().find(|p| p.index == pf.player_index)
                .map(|p| p.name.as_str()).unwrap_or("?");
            eprintln!("  {} pos={:?} boost={:?}", name, pf.position, pf.boost_amount);
        }
    }

    let duration = game_frames.last().map(|f| f.time).unwrap_or(0.0);
    let total_frames = game_frames.len();

    Ok(FrameData {
        players: header_players,
        frames: game_frames,
        goals,
        duration_seconds: duration,
        total_frames,
    })
}

fn find_object_id(objects: &[String], pattern: &str) -> Option<i32> {
    objects.iter().position(|name| name == pattern).map(|i| i as i32)
}
