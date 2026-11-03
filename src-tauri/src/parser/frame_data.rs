use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FrameData {
    pub players: Vec<FramePlayer>,
    pub frames: Vec<GameFrame>,
    pub goals: Vec<GoalEvent>,
    /// Ball touches detected from ball velocity changes with a car nearby.
    pub touches: Vec<TouchEvent>,
    pub demos: Vec<DemoEvent>,
    /// Moments the kickoff countdown reached zero.
    pub kickoffs: Vec<KickoffEvent>,
    pub duration_seconds: f32,
    pub total_frames: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FramePlayer {
    pub index: usize,
    pub name: String,
    pub team: u8, // 0 = blue, 1 = orange
}

#[derive(Debug, Clone, Serialize)]
pub struct GameFrame {
    pub time: f32,
    pub ball: Option<Position>,
    pub ball_kinematics: Option<Kinematics>,
    pub players: Vec<PlayerFrame>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

/// Rigid body state beyond position. Velocities are in unreal units per second.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Kinematics {
    pub rotation: Quaternion,
    pub linear_velocity: Option<Position>,
    pub angular_velocity: Option<Position>,
    pub sleeping: bool,
}

/// Replicated controller and car component state. Every field is optional
/// because replays only replicate these when they change, and older replays
/// may not carry all of them.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct Controls {
    /// -1.0 (full reverse) to 1.0 (full forward)
    pub throttle: Option<f32>,
    /// -1.0 (full left) to 1.0 (full right)
    pub steer: Option<f32>,
    pub handbrake: Option<bool>,
    pub boost_active: Option<bool>,
    pub jump_active: Option<bool>,
    pub double_jump_active: Option<bool>,
    pub dodge_active: Option<bool>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct PlayerFrame {
    pub player_index: usize,
    pub position: Option<Position>,
    pub boost_amount: Option<u8>, // 0-255
    pub kinematics: Option<Kinematics>,
    pub controls: Controls,
}

#[derive(Debug, Clone, Serialize)]
pub struct GoalEvent {
    pub frame: usize,
    pub time: f32,
    /// 0 = blue, 1 = orange
    pub scoring_team: Option<u8>,
    /// Header player index of the scorer
    pub scorer_index: Option<usize>,
    /// Ball position at the goal frame
    pub ball_position: Option<Position>,
    /// Last touches leading to the goal (most recent first, up to 3)
    pub last_touches: Vec<GoalTouch>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GoalTouch {
    pub player_index: usize,
    pub team: u8,
    pub time: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TouchEvent {
    /// Network frame index (not the sampled `frames` index)
    pub frame: usize,
    pub time: f32,
    pub player_index: usize,
    pub team: u8,
    /// Ball speed change across the touch, uu/s
    pub ball_speed_delta: f32,
    /// Ball velocity after the touch (uu/s)
    pub ball_velocity: Option<Position>,
    /// True if the ball is heading toward either goal after this touch
    pub is_shot: bool,
    /// True if a car from the other team is within TOUCH_MAX_DISTANCE within ±0.25s
    pub contested: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DemoEvent {
    pub frame: usize,
    pub time: f32,
    pub attacker_index: Option<usize>,
    pub victim_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KickoffEvent {
    pub frame: usize,
    pub time: f32,
}
