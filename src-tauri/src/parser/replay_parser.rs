use std::collections::HashMap;

use boxcars::{Attribute, HeaderProp, ParserBuilder};

use super::frame_data::*;

/// Parse a .replay file into our simplified frame data format.
pub fn parse_replay(data: &[u8]) -> Result<FrameData, String> {
    let replay = ParserBuilder::new(data)
        .on_error_check_crc()
        .must_parse_network_data()
        .parse()
        .map_err(|e| format!("Failed to parse replay: {:?}", e))?;

    let network_frames = replay
        .network_frames
        .as_ref()
        .ok_or("No network data in replay")?;

    let objects = &replay.objects;

    // Build object name lookup
    let car_obj_ids: Vec<i32> = objects
        .iter()
        .enumerate()
        .filter(|(_, name)| {
            name.contains("Car_Default")
                || name.contains("Car_TA")
                || name.contains("Archetypes.Car")
        })
        .map(|(i, _)| i as i32)
        .collect();

    let ball_obj_ids: Vec<i32> = objects
        .iter()
        .enumerate()
        .filter(|(_, name)| name.contains("Ball_Default") || name.contains("Ball_TA"))
        .map(|(i, _)| i as i32)
        .collect();

    // Find attribute object IDs for the properties we care about
    let _rigid_body_id = find_object_id(objects, "TAGame.RBActor_TA:ReplicatedRBState");
    let _boost_amount_id = find_object_id(objects, "TAGame.CarComponent_Boost_TA:ReplicatedBoostAmount");
    let player_name_id = find_object_id(objects, "Engine.PlayerReplicationInfo:PlayerName");
    let team_id = find_object_id(objects, "Engine.PlayerReplicationInfo:Team");
    let pri_car_id = find_object_id(objects, "TAGame.CarComponent_TA:Vehicle");
    let car_pri_id = find_object_id(objects, "TAGame.PRI_TA:MatchCar"); // PRI -> Car link

    // State tracking
    let mut actor_positions: HashMap<i32, Position> = HashMap::new();
    let mut actor_boost: HashMap<i32, u8> = HashMap::new();
    let mut actor_types: HashMap<i32, ActorType> = HashMap::new();
    let mut player_names: HashMap<i32, String> = HashMap::new(); // PRI actor -> name
    let mut player_teams: HashMap<i32, u8> = HashMap::new(); // PRI actor -> team
    let mut car_to_pri: HashMap<i32, i32> = HashMap::new(); // car actor -> PRI actor
    let mut ball_actor_id: Option<i32> = None;

    let mut game_frames: Vec<GameFrame> = Vec::new();
    let mut goals: Vec<GoalEvent> = Vec::new();

    // Extract goals from tick marks
    for tick in &replay.tick_marks {
        if tick.description.contains("Goal") {
            goals.push(GoalEvent {
                frame: tick.frame as usize,
                time: 0.0, // filled later
            });
        }
    }

    // Process frames
    for (frame_idx, frame) in network_frames.frames.iter().enumerate() {
        // Process new actors
        for new_actor in &frame.new_actors {
            let obj_id = new_actor.object_id.0;
            let actor_id = new_actor.actor_id.0;

            if car_obj_ids.contains(&obj_id) {
                actor_types.insert(actor_id, ActorType::Car);
                if let Some(loc) = &new_actor.initial_trajectory.location {
                    actor_positions.insert(
                        actor_id,
                        Position {
                            x: loc.x as f32 / 100.0,
                            y: loc.y as f32 / 100.0,
                            z: loc.z as f32 / 100.0,
                        },
                    );
                }
            } else if ball_obj_ids.contains(&obj_id) {
                actor_types.insert(actor_id, ActorType::Ball);
                ball_actor_id = Some(actor_id);
                if let Some(loc) = &new_actor.initial_trajectory.location {
                    actor_positions.insert(
                        actor_id,
                        Position {
                            x: loc.x as f32 / 100.0,
                            y: loc.y as f32 / 100.0,
                            z: loc.z as f32 / 100.0,
                        },
                    );
                }
            } else {
                // Could be a PRI or boost component
                let obj_name = objects.get(obj_id as usize).map(|s| s.as_str()).unwrap_or("");
                if obj_name.contains("PRI_TA") || obj_name.contains("PlayerReplicationInfo") {
                    actor_types.insert(actor_id, ActorType::PlayerInfo);
                } else if obj_name.contains("Boost") {
                    actor_types.insert(actor_id, ActorType::BoostComponent);
                }
            }
        }

        // Process deleted actors
        for deleted in &frame.deleted_actors {
            let id = deleted.0;
            actor_types.remove(&id);
            actor_positions.remove(&id);
            actor_boost.remove(&id);
            if ball_actor_id == Some(id) {
                ball_actor_id = None;
            }
        }

        // Process updated actors
        for update in &frame.updated_actors {
            let actor_id = update.actor_id.0;
            let obj_id = update.object_id.0;

            match &update.attribute {
                Attribute::RigidBody(rb) => {
                    actor_positions.insert(
                        actor_id,
                        Position {
                            x: rb.location.x,
                            y: rb.location.y,
                            z: rb.location.z,
                        },
                    );
                }
                Attribute::ReplicatedBoost(boost) => {
                    // The boost is on a boost component actor
                    // We need to find which car this boost component belongs to
                    actor_boost.insert(actor_id, boost.boost_amount);
                }
                Attribute::String(name) if Some(obj_id) == player_name_id => {
                    player_names.insert(actor_id, name.clone());
                }
                Attribute::ActiveActor(aa) if Some(obj_id) == team_id => {
                    // Team actor: 0 = blue, 1 = orange (simplified)
                    player_teams.insert(actor_id, if aa.actor.0 % 2 == 0 { 0 } else { 1 });
                }
                Attribute::ActiveActor(aa) if Some(obj_id) == pri_car_id => {
                    // Boost component -> Car link
                    car_to_pri.insert(actor_id, aa.actor.0);
                }
                Attribute::ActiveActor(aa) if Some(obj_id) == car_pri_id => {
                    // PRI -> Car link
                    car_to_pri.insert(aa.actor.0, actor_id);
                }
                _ => {}
            }
        }

        // Build frame snapshot (sample every 3 frames to reduce data)
        if frame_idx % 3 == 0 {
            let ball_pos = ball_actor_id.and_then(|id| actor_positions.get(&id).copied());

            let mut player_frames = Vec::new();
            for (&actor_id, actor_type) in &actor_types {
                if *actor_type == ActorType::Car {
                    let pos = actor_positions.get(&actor_id).copied();
                    // Try to find boost for this car
                    let boost = actor_boost.get(&actor_id).copied();
                    player_frames.push(PlayerFrame {
                        player_index: actor_id as usize, // temporary, mapped later
                        position: pos,
                        boost_amount: boost,
                    });
                }
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
        if let Some(gf) = game_frames.iter().find(|f| {
            network_frames
                .frames
                .get(goal.frame)
                .map_or(false, |nf| (nf.time - f.time).abs() < 0.2)
        }) {
            goal.time = gf.time;
        }
    }

    // Extract player info from header properties
    let mut frame_players: Vec<FramePlayer> = Vec::new();
    if let Some(HeaderProp::Array(player_stats)) =
        replay.properties.iter().find(|(k, _)| k == "PlayerStats").map(|(_, v)| v)
    {
        for (idx, player) in player_stats.iter().enumerate() {
            let name = player
                .iter()
                .find(|(k, _)| k == "Name")
                .and_then(|(_, v)| {
                    if let HeaderProp::Str(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| format!("Player {}", idx));

            let team = player
                .iter()
                .find(|(k, _)| k == "Team")
                .and_then(|(_, v)| {
                    if let HeaderProp::Int(t) = v {
                        Some(*t as u8)
                    } else {
                        None
                    }
                })
                .unwrap_or(0);

            frame_players.push(FramePlayer {
                index: idx,
                name,
                team,
            });
        }
    }

    let duration = game_frames.last().map(|f| f.time).unwrap_or(0.0);
    let total_frames = game_frames.len();

    Ok(FrameData {
        players: frame_players,
        frames: game_frames,
        goals,
        duration_seconds: duration,
        total_frames,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActorType {
    Car,
    Ball,
    PlayerInfo,
    BoostComponent,
}

fn find_object_id(objects: &[String], pattern: &str) -> Option<i32> {
    objects
        .iter()
        .position(|name| name == pattern)
        .map(|i| i as i32)
}
