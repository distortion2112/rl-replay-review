use std::collections::HashMap;

use boxcars::{Attribute, HeaderProp, ParserBuilder};

use super::frame_data::*;

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

    let obj_name = |id: i32| -> &str {
        objects.get(id as usize).map(|s| s.as_str()).unwrap_or("")
    };

    // Exact archetype IDs
    let car_archetype = find_object_id(objects, "Archetypes.Car.Car_Default");
    let ball_archetype = find_object_id(objects, "Archetypes.Ball.Ball_Default");
    let boost_archetype = find_object_id(objects, "Archetypes.CarComponents.CarComponent_Boost");

    // Attribute object IDs
    let player_name_attr = find_object_id(objects, "Engine.PlayerReplicationInfo:PlayerName");
    let team_attr = find_object_id(objects, "Engine.PlayerReplicationInfo:Team");
    let vehicle_attr = find_object_id(objects, "TAGame.CarComponent_TA:Vehicle");

    eprintln!("Archetypes: car={:?} ball={:?} boost={:?}", car_archetype, ball_archetype, boost_archetype);
    eprintln!("Attrs: name={:?} team={:?} vehicle={:?}", player_name_attr, team_attr, vehicle_attr);

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

    // State tracking
    let mut actor_positions: HashMap<i32, Position> = HashMap::new();
    let mut car_actors: Vec<i32> = Vec::new();
    let mut ball_actor_id: Option<i32> = None;
    let mut boost_comp_actors: Vec<i32> = Vec::new();

    // PRI tracking
    let mut pri_name: HashMap<i32, String> = HashMap::new();
    let mut pri_team: HashMap<i32, u8> = HashMap::new();

    // Linkage: boost component -> car (via Vehicle attr)
    let mut boost_to_car: HashMap<i32, i32> = HashMap::new();
    // Boost amount on component actors
    let mut boost_amount: HashMap<i32, u8> = HashMap::new();

    // We'll discover car<->PRI links by tracking ALL ActiveActor updates
    // and correlating: if a PRI-like actor sends an ActiveActor pointing to a known car, that's a link
    // Also: if a car-like actor sends an ActiveActor pointing to a known PRI, that's a link
    let mut car_to_pri: HashMap<i32, i32> = HashMap::new();

    // Track which actors are PRIs (by seeing PlayerName updates on them)
    let mut known_pris: Vec<i32> = Vec::new();

    let mut game_frames: Vec<GameFrame> = Vec::new();
    let mut goals: Vec<GoalEvent> = Vec::new();

    for tick in &replay.tick_marks {
        if tick.description.contains("Goal") {
            goals.push(GoalEvent { frame: tick.frame as usize, time: 0.0 });
        }
    }

    // First pass: discover all actors and relationships
    for frame in &network_frames.frames {
        for na in &frame.new_actors {
            let aid = na.actor_id.0;
            let oid = na.object_id.0;

            if Some(oid) == car_archetype {
                if !car_actors.contains(&aid) {
                    car_actors.push(aid);
                }
            } else if Some(oid) == ball_archetype {
                ball_actor_id = Some(aid);
            } else if Some(oid) == boost_archetype {
                if !boost_comp_actors.contains(&aid) {
                    boost_comp_actors.push(aid);
                }
            }

            if let Some(loc) = &na.initial_trajectory.location {
                actor_positions.insert(aid, Position {
                    x: loc.x as f32,
                    y: loc.y as f32,
                    z: loc.z as f32,
                });
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
                }
                Attribute::String(name) if Some(oid) == player_name_attr => {
                    pri_name.insert(aid, name.clone());
                    if !known_pris.contains(&aid) {
                        known_pris.push(aid);
                    }
                }
                Attribute::ActiveActor(aa) if Some(oid) == team_attr => {
                    // Team: check the object name of the team actor
                    let team_name = obj_name(aa.actor.0);
                    let team = if team_name.contains("1") || team_name.contains("Orange") { 1u8 } else { 0u8 };
                    pri_team.insert(aid, team);
                }
                Attribute::ActiveActor(aa) if Some(oid) == vehicle_attr => {
                    // Boost component `aid` -> car `aa.actor`
                    if aa.active {
                        boost_to_car.insert(aid, aa.actor.0);
                    }
                }
                Attribute::ActiveActor(aa) if aa.active => {
                    // Generic ActiveActor: check if this links a car to a PRI or vice versa
                    let target = aa.actor.0;
                    if car_actors.contains(&aid) && known_pris.contains(&target) {
                        car_to_pri.insert(aid, target);
                    } else if known_pris.contains(&aid) && car_actors.contains(&target) {
                        car_to_pri.insert(target, aid);
                    }
                }
                Attribute::ReplicatedBoost(boost) => {
                    boost_amount.insert(aid, boost.boost_amount);
                }
                _ => {}
            }
        }
    }

    eprintln!("Discovery: {} cars, {} PRIs, {} boost comps, {} car->pri links",
        car_actors.len(), known_pris.len(), boost_comp_actors.len(), car_to_pri.len());
    for (&car, &pri) in &car_to_pri {
        eprintln!("  car {} -> PRI {} (name={:?})", car, pri, pri_name.get(&pri));
    }

    // Build car -> header player index
    let mut car_to_player: HashMap<i32, usize> = HashMap::new();
    for (&car_id, &pri_id) in &car_to_pri {
        if let Some(name) = pri_name.get(&pri_id) {
            if let Some(hp) = header_players.iter().find(|p| p.name == *name) {
                car_to_player.insert(car_id, hp.index);
            }
        }
    }

    // Second pass: extract frame snapshots
    actor_positions.clear();
    ball_actor_id = None;
    let mut live_boost_amount: HashMap<i32, u8> = HashMap::new();
    let mut live_boost_to_car: HashMap<i32, i32> = HashMap::new();
    let mut alive_cars: Vec<i32> = Vec::new();

    for (frame_idx, frame) in network_frames.frames.iter().enumerate() {
        for na in &frame.new_actors {
            let aid = na.actor_id.0;
            let oid = na.object_id.0;
            if Some(oid) == ball_archetype {
                ball_actor_id = Some(aid);
            }
            if Some(oid) == car_archetype {
                if !alive_cars.contains(&aid) {
                    alive_cars.push(aid);
                }
            }
            if let Some(loc) = &na.initial_trajectory.location {
                actor_positions.insert(aid, Position {
                    x: loc.x as f32,
                    y: loc.y as f32,
                    z: loc.z as f32,
                });
            }
        }

        for deleted in &frame.deleted_actors {
            let id = deleted.0;
            actor_positions.remove(&id);
            alive_cars.retain(|c| *c != id);
            live_boost_to_car.remove(&id);
            live_boost_amount.remove(&id);
            if ball_actor_id == Some(id) {
                ball_actor_id = None;
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
                }
                Attribute::ReplicatedBoost(boost) => {
                    live_boost_amount.insert(aid, boost.boost_amount);
                }
                Attribute::ActiveActor(aa) if Some(oid) == vehicle_attr => {
                    if aa.active {
                        live_boost_to_car.insert(aid, aa.actor.0);
                    }
                }
                _ => {}
            }
        }

        // Sample every 3 frames
        if frame_idx % 3 == 0 {
            let ball_pos = ball_actor_id.and_then(|id| actor_positions.get(&id).copied());

            // Build live car -> boost lookup
            let mut car_boost: HashMap<i32, u8> = HashMap::new();
            for (comp, car) in &live_boost_to_car {
                if let Some(amount) = live_boost_amount.get(comp) {
                    car_boost.insert(*car, *amount);
                }
            }

            let mut player_frames = Vec::new();
            let mut seen_players: Vec<usize> = Vec::new();
            for &car_id in &alive_cars {
                let player_index = car_to_player.get(&car_id).copied().unwrap_or(usize::MAX);
                if player_index == usize::MAX || seen_players.contains(&player_index) {
                    continue;
                }
                seen_players.push(player_index);

                let pos = actor_positions.get(&car_id).copied();
                let boost = car_boost.get(&car_id).copied();

                player_frames.push(PlayerFrame {
                    player_index,
                    position: pos,
                    boost_amount: boost,
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

fn find_object_id_contains(objects: &[String], pattern: &str) -> Option<i32> {
    objects.iter().position(|name| name.contains(pattern)).map(|i| i as i32)
}
