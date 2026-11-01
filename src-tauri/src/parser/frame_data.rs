use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FrameData {
    pub players: Vec<FramePlayer>,
    pub frames: Vec<GameFrame>,
    pub goals: Vec<GoalEvent>,
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
    pub players: Vec<PlayerFrame>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct PlayerFrame {
    pub player_index: usize,
    pub position: Option<Position>,
    pub boost_amount: Option<u8>, // 0-255
}

#[derive(Debug, Clone, Serialize)]
pub struct GoalEvent {
    pub frame: usize,
    pub time: f32,
}
