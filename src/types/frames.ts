export interface FrameData {
  players: FramePlayer[];
  frames: GameFrame[];
  goals: GoalEvent[];
  touches?: TouchEvent[];
  demos?: DemoEvent[];
  kickoffs?: KickoffEvent[];
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
  ball_kinematics?: Kinematics | null;
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
  kinematics?: Kinematics | null;
  controls?: Controls;
}

export interface Quaternion {
  x: number;
  y: number;
  z: number;
  w: number;
}

/** Rigid body state beyond position. Velocities are in uu/s. */
export interface Kinematics {
  rotation: Quaternion;
  linear_velocity: Position | null;
  angular_velocity: Position | null;
  sleeping: boolean;
}

/** Replicated controller and car component state. */
export interface Controls {
  throttle: number | null; // -1 to 1
  steer: number | null; // -1 to 1
  handbrake: boolean | null;
  boost_active: boolean | null;
  jump_active: boolean | null;
  double_jump_active: boolean | null;
  dodge_active: boolean | null;
}

export interface GoalTouch {
  player_index: number;
  team: number;
  time: number;
}

export interface GoalEvent {
  frame: number;
  time: number;
  scoring_team: number | null;
  scorer_index: number | null;
  ball_position: Position | null;
  last_touches: GoalTouch[];
}

export interface TouchEvent {
  frame: number; // network frame index
  time: number;
  player_index: number;
  team: number;
  ball_speed_delta: number;
  ball_velocity: Position | null;
  is_shot: boolean;
  contested: boolean;
}

export interface DemoEvent {
  frame: number;
  time: number;
  attacker_index: number | null;
  victim_index: number | null;
}

export interface KickoffEvent {
  frame: number;
  time: number;
}
