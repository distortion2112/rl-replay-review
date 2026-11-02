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
                players: vec![
                    PlayerFrame { player_index: 0, position: Some(Position { x: -1000.0, y: -3000.0, z: 17.0 }), boost_amount: Some(33) },
                    PlayerFrame { player_index: 1, position: Some(Position { x: 1000.0, y: 3000.0, z: 17.0 }), boost_amount: None },
                ],
            },
        ],
        goals: vec![GoalEvent { frame: 100, time: 45.5 }],
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
