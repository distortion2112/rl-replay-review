export interface FrameData {
  players: FramePlayer[];
  frames: GameFrame[];
  goals: GoalEvent[];
  duration_seconds: number;
  total_frames: number;
}

export interface FramePlayer {
  index: number;
  name: string;
  team: number; // 0 = blue, 1 = orange
}

export interface GameFrame {
  time: number;
  ball: Position | null;
  players: PlayerFrame[];
}

export interface Position {
  x: number;
  y: number;
  z: number;
}

export interface PlayerFrame {
  player_index: number;
  position: Position | null;
  boost_amount: number | null;
}

export interface GoalEvent {
  frame: number;
  time: number;
}
