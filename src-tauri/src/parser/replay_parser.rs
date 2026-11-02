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
    // car -> player map attributes positions, touches and demos to the wrong players.
    let pawn_pri_attr = find_object_id(objects, "Engine.Pawn:PlayerReplicationInfo");

    // Controls and car component state
    let throttle_attr = find_object_id(objects, "TAGame.Vehicle_TA:ReplicatedThrottle");
    let steer_attr = find_object_id(objects, "TAGame.Vehicle_TA:ReplicatedSteer");
    let handbrake_attr = find_object_id(objects, "TAGame.Vehicle_TA:bReplicatedHandbrake");
    let comp_active_attr = find_object_id(objects, "TAGame.CarComponent_TA:ReplicatedActive");
    let countdown_attr = objects
        .iter()
        .position(|name| name.ends_with(":ReplicatedRoundCountDownNumber"))
        .map(|i| i as i32);
    let jump_archetype = find_object_id(objects, "Archetypes.CarComponents.CarComponent_Jump");
    let double_jump_archetype =
        find_object_id(objects, "Archetypes.CarComponents.CarComponent_DoubleJump");
    let dodge_archetype = find_object_id(objects, "Archetypes.CarComponents.CarComponent_Dodge");

    let mut goals: Vec<GoalEvent> = Vec::new();
    for tick in &replay.tick_marks {
        if tick.description.contains("Goal") {
            goals.push(GoalEvent { frame: tick.frame as usize, time: 0.0 });
        }
    }

    let mut game_frames: Vec<GameFrame> = Vec::new();
    let mut actor_positions: HashMap<i32, Position> = HashMap::new();
    let mut actor_kinematics: HashMap<i32, Kinematics> = HashMap::new();
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
    // Any car component (boost, jump, double jump, dodge) -> car
    let mut live_comp_to_car: HashMap<i32, i32> = HashMap::new();
    let mut comp_kind: HashMap<i32, ComponentKind> = HashMap::new();
    let mut comp_active: HashMap<i32, bool> = HashMap::new();
    let mut car_controls: HashMap<i32, Controls> = HashMap::new();
    // Last player each live car resolved to. A demolished car's Pawn PRI is unset in
    // the same frame as the demo, so events fall back to this. Frames do not.
    let mut car_last_player: HashMap<i32, usize> = HashMap::new();

    let mut touch_detector = TouchDetector::default();
    let mut touches: Vec<TouchEvent> = Vec::new();
    let mut demos: Vec<DemoEvent> = Vec::new();
    let mut kickoffs: Vec<KickoffEvent> = Vec::new();
    let mut last_countdown: Option<i32> = None;

    for (frame_idx, frame) in network_frames.frames.iter().enumerate() {
        // Deleted actors, and actors re-created under an existing ID (respawned cars are
        // sometimes replaced without a delete), have all state cleared before new actors
        // are processed, so a recycled ID starts clean.
        let created: Vec<i32> = frame.new_actors.iter().map(|na| na.actor_id.0).collect();
        for id in frame.deleted_actors.iter().map(|d| d.0).chain(created.iter().copied()) {
            actor_positions.remove(&id);
            actor_kinematics.remove(&id);
            actor_motion.remove(&id);
            alive_cars.retain(|c| *c != id);
            live_car_to_pri.remove(&id);
            live_pri_name.remove(&id);
            live_comp_to_car.remove(&id);
            live_boost_amount.remove(&id);
            comp_kind.remove(&id);
            comp_active.remove(&id);
            car_controls.remove(&id);
            car_last_player.remove(&id);
            if ball_actor_id == Some(id) {
                ball_actor_id = None;
                touch_detector.reset();
            }
        }

        for na in &frame.new_actors {
            let aid = na.actor_id.0;
            let oid = na.object_id.0;
            if Some(oid) == ball_archetype {
                ball_actor_id = Some(aid);
                touch_detector.reset();
            }
            if Some(oid) == car_archetype && !alive_cars.contains(&aid) {
                alive_cars.push(aid);
            }
            let kind = if Some(oid) == boost_archetype {
                Some(ComponentKind::Boost)
            } else if Some(oid) == jump_archetype {
                Some(ComponentKind::Jump)
            } else if Some(oid) == double_jump_archetype {
                Some(ComponentKind::DoubleJump)
            } else if Some(oid) == dodge_archetype {
                Some(ComponentKind::Dodge)
            } else {
                None
            };
            if let Some(kind) = kind {
                comp_kind.insert(aid, kind);
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
                    actor_positions.insert(aid, vec3(&rb.location));
                    actor_kinematics.insert(aid, Kinematics {
                        rotation: Quaternion {
                            x: rb.rotation.x,
                            y: rb.rotation.y,
                            z: rb.rotation.z,
                            w: rb.rotation.w,
                        },
                        linear_velocity: rb.linear_velocity.as_ref().map(vec3),
                        angular_velocity: rb.angular_velocity.as_ref().map(vec3),
                        sleeping: rb.sleeping,
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
                        live_comp_to_car.insert(aid, aa.actor.0);
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
                Attribute::Byte(b) if Some(oid) == throttle_attr => {
                    car_controls.entry(aid).or_default().throttle = Some(axis_from_byte(*b));
                }
                Attribute::Byte(b) if Some(oid) == steer_attr => {
                    car_controls.entry(aid).or_default().steer = Some(axis_from_byte(*b));
                }
                Attribute::Boolean(on) if Some(oid) == handbrake_attr => {
                    car_controls.entry(aid).or_default().handbrake = Some(*on);
                }
                Attribute::Byte(b) if Some(oid) == comp_active_attr => {
                    // Odd values mean the component is active
                    comp_active.insert(aid, b % 2 == 1);
                }
                Attribute::Int(n) if Some(oid) == countdown_attr => {
                    if *n == 0 && last_countdown.is_some_and(|prev| prev > 0) {
                        kickoffs.push(KickoffEvent { frame: frame_idx, time: frame.time });
                    }
                    last_countdown = Some(*n);
                }
                // A newly created car's initial replication repeats the last demo
                // recorded on that ID, so demo attributes on a car created this frame
                // are stale. Current replays replicate demos only as DemolishExtended.
                Attribute::Demolish(demo) if !created.contains(&aid) => {
                    let link = PlayerLink::new(&live_car_to_pri, &live_pri_name, &header_players, &car_last_player);
                    push_demo(&mut demos, &link, frame_idx, frame.time, demo.attacker.0, demo.victim.0);
                }
                Attribute::DemolishFx(demo) if !created.contains(&aid) => {
                    let link = PlayerLink::new(&live_car_to_pri, &live_pri_name, &header_players, &car_last_player);
                    push_demo(&mut demos, &link, frame_idx, frame.time, demo.attacker.0, demo.victim.0);
                }
                Attribute::DemolishExtended(demo) if !created.contains(&aid) => {
                    let link = PlayerLink::new(&live_car_to_pri, &live_pri_name, &header_players, &car_last_player);
                    push_demo(&mut demos, &link, frame_idx, frame.time, demo.attacker.actor.0, demo.victim.actor.0);
                }
                _ => {}
            }
        }

        let link = PlayerLink::new(&live_car_to_pri, &live_pri_name, &header_players, &car_last_player);

        // Touch detection runs at full network rate, not the sampled rate
        if let Some(ball_id) = ball_actor_id
            && let (Some(ball_pos), Some(ball_kin)) =
                (actor_positions.get(&ball_id), actor_kinematics.get(&ball_id))
            && let Some(delta) = touch_detector.observe(frame.time, ball_kin.linear_velocity)
        {
            let nearest = alive_cars
                .iter()
                .filter_map(|car| {
                    let player = link.player_index(*car)?;
                    let pos = actor_positions.get(car)?;
                    Some((player, distance(pos, ball_pos)))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((player_index, dist)) = nearest
                && dist < TOUCH_MAX_DISTANCE
            {
                let duplicate = touches.last().is_some_and(|t| {
                    t.player_index == player_index && frame.time - t.time < TOUCH_MERGE_SECONDS
                });
                if !duplicate {
                    let team = header_players
                        .iter()
                        .find(|p| p.index == player_index)
                        .map(|p| p.team)
                        .unwrap_or(0);
                    touches.push(TouchEvent {
                        frame: frame_idx,
                        time: frame.time,
                        player_index,
                        team,
                        ball_speed_delta: delta,
                    });
                }
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
            let ball_kin = ball_actor_id.and_then(|id| actor_kinematics.get(&id).copied());

            // Build live car -> boost and component-state lookups
            let mut car_boost: HashMap<i32, u8> = HashMap::new();
            let mut car_comp_controls: HashMap<i32, Controls> = HashMap::new();
            for (comp, car) in &live_comp_to_car {
                if let Some(amount) = live_boost_amount.get(comp) {
                    car_boost.insert(*car, *amount);
                }
                if let Some(kind) = comp_kind.get(comp) {
                    let active = comp_active.get(comp).copied().unwrap_or(false);
                    let c = car_comp_controls.entry(*car).or_default();
                    match kind {
                        ComponentKind::Boost => c.boost_active = Some(active),
                        ComponentKind::Jump => c.jump_active = Some(active),
                        ComponentKind::DoubleJump => c.double_jump_active = Some(active),
                        ComponentKind::Dodge => c.dodge_active = Some(active),
                    }
                }
            }

            let mut player_frames = Vec::new();
            let mut seen_players: Vec<usize> = Vec::new();
            for &car_id in &alive_cars {
                let Some(player_index) = link.player_index(car_id) else { continue };
                if seen_players.contains(&player_index) {
                    continue;
                }
                seen_players.push(player_index);

                let mut controls = car_controls.get(&car_id).copied().unwrap_or_default();
                if let Some(comp) = car_comp_controls.get(&car_id) {
                    controls.boost_active = comp.boost_active;
                    controls.jump_active = comp.jump_active;
                    controls.double_jump_active = comp.double_jump_active;
                    controls.dodge_active = comp.dodge_active;
                }

                player_frames.push(PlayerFrame {
                    player_index,
                    position: position_at(car_id),
                    boost_amount: car_boost.get(&car_id).copied(),
                    kinematics: actor_kinematics.get(&car_id).copied(),
                    controls,
                });
            }

            game_frames.push(GameFrame {
                time: frame.time,
                ball: ball_pos,
                ball_kinematics: ball_kin,
                players: player_frames,
            });
        }

        let resolved: Vec<(i32, usize)> = alive_cars
            .iter()
            .filter_map(|&car| Some((car, link.player_index(car)?)))
            .collect();
        car_last_player.extend(resolved);
    }

    eprintln!("Events: {} touches, {} demos, {} kickoffs", touches.len(), demos.len(), kickoffs.len());

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
        touches,
        demos,
        kickoffs,
        duration_seconds: duration,
        total_frames,
    })
}

fn find_object_id(objects: &[String], pattern: &str) -> Option<i32> {
    objects.iter().position(|name| name == pattern).map(|i| i as i32)
}

/// Max ball-center to car-center distance (uu) for a velocity change to count as a touch.
const TOUCH_MAX_DISTANCE: f32 = 300.0;
/// Ball speed change (uu/s, after removing gravity) that counts as a possible touch.
const TOUCH_MIN_SPEED_DELTA: f32 = 250.0;
/// Touches by the same player closer together than this are merged.
const TOUCH_MERGE_SECONDS: f32 = 0.1;
/// Rocket League gravity, uu/s^2.
const GRAVITY_Z: f32 = -650.0;

#[derive(Debug, Clone, Copy)]
enum ComponentKind {
    Boost,
    Jump,
    DoubleJump,
    Dodge,
}

/// Resolves a car actor to a header player index using the links live at this frame.
struct PlayerLink<'a> {
    car_to_pri: &'a HashMap<i32, i32>,
    pri_name: &'a HashMap<i32, String>,
    header: &'a [FramePlayer],
    last_player: &'a HashMap<i32, usize>,
}

impl<'a> PlayerLink<'a> {
    fn new(
        car_to_pri: &'a HashMap<i32, i32>,
        pri_name: &'a HashMap<i32, String>,
        header: &'a [FramePlayer],
        last_player: &'a HashMap<i32, usize>,
    ) -> Self {
        Self { car_to_pri, pri_name, header, last_player }
    }

    /// The player driving `car` right now.
    fn player_index(&self, car: i32) -> Option<usize> {
        let pri = self.car_to_pri.get(&car)?;
        let name = self.pri_name.get(pri)?;
        self.header.iter().find(|p| p.name == *name).map(|p| p.index)
    }

    /// For events: falls back to the car's last known player, since a car's PRI
    /// link is cleared the moment it is demolished.
    fn event_player_index(&self, car: i32) -> Option<usize> {
        self.player_index(car).or_else(|| self.last_player.get(&car).copied())
    }
}

/// Flags frames where the ball's velocity changes by more than gravity explains.
#[derive(Debug, Default)]
struct TouchDetector {
    last: Option<(f32, Position)>,
}

impl TouchDetector {
    fn reset(&mut self) {
        self.last = None;
    }

    /// Returns the unexplained speed change if this observation looks like a touch.
    fn observe(&mut self, time: f32, velocity: Option<Position>) -> Option<f32> {
        let v = velocity?;
        let prev = self.last.replace((time, v));
        let (prev_time, prev_v) = prev?;
        let dt = time - prev_time;
        if dt <= 0.0 {
            return None;
        }
        let dx = v.x - prev_v.x;
        let dy = v.y - prev_v.y;
        let dz = v.z - (prev_v.z + GRAVITY_Z * dt);
        let delta = (dx * dx + dy * dy + dz * dz).sqrt();
        (delta > TOUCH_MIN_SPEED_DELTA).then_some(delta)
    }
}

fn vec3(v: &boxcars::Vector3f) -> Position {
    Position { x: v.x, y: v.y, z: v.z }
}

fn distance(a: &Position, b: &Position) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Maps a replicated 0-255 axis byte (128 = neutral) to -1.0..=1.0.
fn axis_from_byte(b: u8) -> f32 {
    ((f32::from(b) - 128.0) / 127.0).clamp(-1.0, 1.0)
}

/// Records a demolition, skipping re-replications of the same demo.
fn push_demo(
    demos: &mut Vec<DemoEvent>,
    link: &PlayerLink,
    frame: usize,
    time: f32,
    attacker_car: i32,
    victim_car: i32,
) {
    let attacker_index = link.event_player_index(attacker_car);
    let victim_index = link.event_player_index(victim_car);
    let duplicate = demos.iter().rev().take(4).any(|d| {
        d.attacker_index == attacker_index
            && d.victim_index == victim_index
            && time - d.time < 1.0
    });
    if !duplicate {
        demos.push(DemoEvent { frame, time, attacker_index, victim_index });
    }
}
