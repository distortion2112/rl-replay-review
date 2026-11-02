use rl_replay_review_lib::parser::frame_data::*;
use rl_replay_review_lib::parser::replay_parser::parse_replay;

fn load_fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", path, e))
}

// --- Integration tests with real replay ---

#[test]
fn parse_ranked_2v2_succeeds() {
    let data = load_fixture("ranked-2v2.replay");
    let result = parse_replay(&data);
    assert!(result.is_ok(), "Parser failed: {:?}", result.err());
}

#[test]
fn parse_ranked_2v2_has_four_players() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();
    assert_eq!(frame_data.players.len(), 4, "Expected 4 players in 2v2 replay");
}

#[test]
fn parse_ranked_2v2_player_names() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    let names: Vec<&str> = frame_data.players.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"rtl_escolta394"), "Missing player rtl_escolta394");
    assert!(names.contains(&"RobV3107"), "Missing player RobV3107");
    assert!(names.contains(&"JohnDavid1682"), "Missing player JohnDavid1682");
    assert!(names.contains(&"aykildu"), "Missing player aykildu");
}

#[test]
fn parse_ranked_2v2_teams() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    let blue: Vec<&str> = frame_data.players.iter()
        .filter(|p| p.team == 0)
        .map(|p| p.name.as_str())
        .collect();
    let orange: Vec<&str> = frame_data.players.iter()
        .filter(|p| p.team == 1)
        .map(|p| p.name.as_str())
        .collect();

    assert_eq!(blue.len(), 2, "Expected 2 blue players, got: {:?}", blue);
    assert_eq!(orange.len(), 2, "Expected 2 orange players, got: {:?}", orange);
}

#[test]
fn parse_ranked_2v2_has_frames() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    assert!(frame_data.total_frames > 0, "Expected at least some frames");
    assert_eq!(frame_data.total_frames, frame_data.frames.len());
}

#[test]
fn parse_ranked_2v2_duration_reasonable() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    // Match was ~488s with 99s overtime
    assert!(frame_data.duration_seconds > 400.0,
        "Duration too short: {}s", frame_data.duration_seconds);
    assert!(frame_data.duration_seconds < 700.0,
        "Duration too long: {}s", frame_data.duration_seconds);
}

#[test]
fn parse_ranked_2v2_has_goals() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    // Blue 4 - Orange 5 = 9 goals total
    assert_eq!(frame_data.goals.len(), 9,
        "Expected 9 goals, got {}", frame_data.goals.len());
}

#[test]
fn parse_ranked_2v2_goal_times_ascending() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    for window in frame_data.goals.windows(2) {
        assert!(window[1].time >= window[0].time,
            "Goal times not ascending: {} >= {}", window[0].time, window[1].time);
    }
}

#[test]
fn parse_ranked_2v2_frames_have_ball() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    let frames_with_ball = frame_data.frames.iter()
        .filter(|f| f.ball.is_some())
        .count();
    let ratio = frames_with_ball as f64 / frame_data.frames.len() as f64;

    // Ball should be present in most frames (might disappear briefly on goals)
    assert!(ratio > 0.8,
        "Only {:.1}% of frames have ball position", ratio * 100.0);
}

#[test]
fn parse_ranked_2v2_frames_have_players() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    let frames_with_players = frame_data.frames.iter()
        .filter(|f| !f.players.is_empty())
        .count();
    let ratio = frames_with_players as f64 / frame_data.frames.len() as f64;

    assert!(ratio > 0.8,
        "Only {:.1}% of frames have player data", ratio * 100.0);
}

#[test]
fn parse_ranked_2v2_some_boost_data() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    let frames_with_boost = frame_data.frames.iter()
        .filter(|f| f.players.iter().any(|p| p.boost_amount.is_some()))
        .count();

    assert!(frames_with_boost > 0, "No frames contain boost data");
}

#[test]
fn parse_ranked_2v2_positions_in_field_bounds() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    // RL field is roughly +-4100 x, +-5200 y, 0-2000 z
    // Players can extend past walls (goal interior, demos, respawns) so use generous margin
    let margin = 1200.0;
    for frame in &frame_data.frames {
        if let Some(ball) = &frame.ball {
            assert!(ball.x.abs() < 4100.0 + margin, "Ball X out of bounds: {}", ball.x);
            assert!(ball.y.abs() < 5200.0 + margin, "Ball Y out of bounds: {}", ball.y);
        }
        for pf in &frame.players {
            if let Some(pos) = &pf.position {
                assert!(pos.x.abs() < 4100.0 + margin, "Player X out of bounds: {}", pos.x);
                assert!(pos.y.abs() < 5200.0 + margin, "Player Y out of bounds: {}", pos.y);
            }
        }
    }
}

// --- Full capture tests (#32) ---

#[test]
fn parse_ranked_2v2_has_velocities() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    let moving_cars = frame_data.frames.iter()
        .flat_map(|f| f.players.iter())
        .filter_map(|p| p.kinematics.and_then(|k| k.linear_velocity))
        .filter(|v| (v.x * v.x + v.y * v.y + v.z * v.z).sqrt() > 500.0)
        .count();
    assert!(moving_cars > 1000, "Expected plenty of fast car frames, got {}", moving_cars);

    let ball_moving = frame_data.frames.iter()
        .filter_map(|f| f.ball_kinematics.and_then(|k| k.linear_velocity))
        .any(|v| v.x.abs() + v.y.abs() + v.z.abs() > 100.0);
    assert!(ball_moving, "Ball never has a meaningful velocity");
}

#[test]
fn parse_ranked_2v2_quaternions_normalized() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    let mut checked = 0;
    for frame in &frame_data.frames {
        for k in frame.players.iter().filter_map(|p| p.kinematics).chain(frame.ball_kinematics) {
            let q = k.rotation;
            let norm = (q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w).sqrt();
            assert!((norm - 1.0).abs() < 0.05, "Quaternion not normalized: {:?} (norm {})", q, norm);
            checked += 1;
        }
    }
    assert!(checked > 0, "No rotations captured");
}

#[test]
fn parse_ranked_2v2_touch_before_each_goal() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    let n = frame_data.touches.len();
    assert!((50..2000).contains(&n), "Implausible touch count: {}", n);
    for goal in &frame_data.goals {
        let touched = frame_data.touches.iter()
            .any(|t| t.time <= goal.time && goal.time - t.time < 8.0);
        assert!(touched, "No touch in the 8s before the goal at {}s", goal.time);
    }
}

#[test]
fn parse_ranked_2v2_touch_players_valid() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    for t in &frame_data.touches {
        let player = frame_data.players.iter().find(|p| p.index == t.player_index);
        assert!(player.is_some(), "Touch references unknown player {}", t.player_index);
        assert_eq!(player.unwrap().team, t.team, "Touch team does not match player team");
    }
    let touchers: std::collections::HashSet<usize> =
        frame_data.touches.iter().map(|t| t.player_index).collect();
    assert_eq!(touchers.len(), 4, "Expected all 4 players to touch the ball");
}

#[test]
fn parse_ranked_2v2_kickoffs() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    // Opening kickoff plus one after every goal (overtime adds one more)
    assert!(frame_data.kickoffs.len() >= frame_data.goals.len(),
        "Expected at least {} kickoffs, got {}", frame_data.goals.len(), frame_data.kickoffs.len());
    assert!(frame_data.kickoffs.len() <= frame_data.goals.len() + 3,
        "Too many kickoffs: {}", frame_data.kickoffs.len());
}

#[test]
fn parse_ranked_2v2_controls_present() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    let players = frame_data.frames.iter().flat_map(|f| f.players.iter());
    let (mut throttle, mut steer, mut boosting, mut jumping) = (0, 0, 0, 0);
    for p in players {
        let c = p.controls;
        if let Some(t) = c.throttle {
            assert!((-1.0..=1.0).contains(&t), "Throttle out of range: {}", t);
            throttle += 1;
        }
        if c.steer.is_some() { steer += 1; }
        if c.boost_active == Some(true) { boosting += 1; }
        if c.jump_active == Some(true) { jumping += 1; }
    }
    assert!(throttle > 0, "No throttle data");
    assert!(steer > 0, "No steer data");
    assert!(boosting > 0, "Never saw boost active");
    assert!(jumping > 0, "Never saw jump active");
}

#[test]
fn parse_ranked_2v2_demo_players_valid() {
    let data = load_fixture("ranked-2v2.replay");
    let frame_data = parse_replay(&data).unwrap();

    // The fixture may have no demos; when present they must reference real players
    for d in &frame_data.demos {
        if let (Some(a), Some(v)) = (d.attacker_index, d.victim_index) {
            assert_ne!(a, v, "Attacker and victim are the same player");
        }
    }
}

// --- Edge case tests ---

#[test]
fn parse_empty_data_returns_error() {
    let result = parse_replay(&[]);
    assert!(result.is_err(), "Parsing empty data should fail");
}

#[test]
fn parse_garbage_data_returns_error() {
    let garbage = vec![0u8; 1024];
    let result = parse_replay(&garbage);
    assert!(result.is_err(), "Parsing garbage data should fail");
}

#[test]
fn parse_truncated_replay_returns_error() {
    let data = load_fixture("ranked-2v2.replay");
    // Take only the first 100 bytes — clearly incomplete
    let truncated = &data[..100];
    let result = parse_replay(truncated);
    assert!(result.is_err(), "Parsing truncated replay should fail");
}

// --- frame_data struct tests ---

#[test]
fn frame_data_serializes_to_json() {
    let data = FrameData {
        players: vec![
            FramePlayer { index: 0, name: "Alice".into(), team: 0 },
            FramePlayer { index: 1, name: "Bob".into(), team: 1 },
        ],
        frames: vec![
            GameFrame {
                time: 0.0,
                ball: Some(Position { x: 0.0, y: 0.0, z: 100.0 }),
                ball_kinematics: None,
                players: vec![
                    PlayerFrame { player_index: 0, position: Some(Position { x: -1000.0, y: -3000.0, z: 17.0 }), boost_amount: Some(33), kinematics: None, controls: Controls::default() },
                    PlayerFrame { player_index: 1, position: Some(Position { x: 1000.0, y: 3000.0, z: 17.0 }), boost_amount: None, kinematics: None, controls: Controls::default() },
                ],
            },
        ],
        goals: vec![GoalEvent { frame: 100, time: 45.5 }],
        touches: vec![],
        demos: vec![],
        kickoffs: vec![],
        duration_seconds: 300.0,
        total_frames: 1,
    };

    let json = serde_json::to_string(&data);
    assert!(json.is_ok(), "FrameData should serialize to JSON");

    let json_str = json.unwrap();
    assert!(json_str.contains("Alice"));
    assert!(json_str.contains("Bob"));
    assert!(json_str.contains("45.5"));
}

#[test]
fn position_copy_semantics() {
    let pos = Position { x: 1.0, y: 2.0, z: 3.0 };
    let pos2 = pos; // Copy
    assert_eq!(pos.x, pos2.x);
    assert_eq!(pos.y, pos2.y);
    assert_eq!(pos.z, pos2.z);
}

#[test]
fn parse_ranked_2v2_players_tracked_across_car_respawns() {
    // Car actor IDs are recycled across respawns and can move between players.
    // A full-match player must appear in nearly every frame; only demo respawns
    // (~3s each) should leave gaps.
    let data = load_fixture("ranked-2v2.replay");
    let fd = parse_replay(&data).unwrap();
    let mut present = vec![0usize; fd.players.len()];
    for frame in &fd.frames {
        for p in &frame.players {
            present[p.player_index] += 1;
        }
    }
    for (i, count) in present.iter().enumerate() {
        let ratio = *count as f64 / fd.frames.len() as f64;
        assert!(ratio > 0.9, "{} present in only {:.0}% of frames", fd.players[i].name, ratio * 100.0);
    }
}

#[test]
fn parse_ranked_2v2_goal_credited_to_last_scoring_team_touch() {
    // Rocket League credits a goal to the last touch by the scoring team. Touches
    // attributed through stale car->player links fail this.
    use boxcars::{HeaderProp, ParserBuilder};
    let data = load_fixture("ranked-2v2.replay");
    let fd = parse_replay(&data).unwrap();
    let header = ParserBuilder::new(&data).never_parse_network_data().parse().unwrap();
    let Some((_, HeaderProp::Array(goals))) = header.properties.iter().find(|(k, _)| k == "Goals") else {
        panic!("Fixture has no Goals header");
    };
    assert!(!goals.is_empty());
    for g in goals {
        let frame = g.iter().find_map(|(k, v)| match (k.as_str(), v) {
            ("frame", HeaderProp::Int(f)) => Some(*f as usize),
            _ => None,
        }).unwrap();
        let scorer = g.iter().find_map(|(k, v)| match (k.as_str(), v) {
            ("PlayerName", HeaderProp::Str(s)) => Some(s.clone()),
            _ => None,
        }).unwrap();
        let team = fd.players.iter().find(|p| p.name == scorer).unwrap().team;
        let last = fd.touches.iter().rev().find(|t| t.frame <= frame && t.team == team).unwrap();
        assert_eq!(fd.players[last.player_index].name, scorer, "Goal at frame {} credited to wrong player", frame);
    }
}

#[test]
fn parse_ranked_2v2_demos_unique_with_victims() {
    // A demolished car's PRI link is cleared in the demo frame, and a respawned car's
    // initial replication repeats the last demo. Neither may lose the victim or
    // produce a duplicate.
    let data = load_fixture("ranked-2v2.replay");
    let fd = parse_replay(&data).unwrap();
    assert!(!fd.demos.is_empty(), "No demos detected");
    for d in &fd.demos {
        assert!(d.victim_index.is_some(), "Demo at {:.1}s has no victim", d.time);
    }
    for pair in fd.demos.windows(2) {
        let same = pair[0].attacker_index == pair[1].attacker_index && pair[0].victim_index == pair[1].victim_index;
        assert!(!(same && pair[1].time - pair[0].time < 5.0), "Duplicate demo at {:.1}s and {:.1}s", pair[0].time, pair[1].time);
    }
}
