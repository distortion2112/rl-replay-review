import type { GameFrame, PlayerFrame, Position } from "../types/frames";

/**
 * Faster than anything in Rocket League can legitimately move (ball max is
 * ~6000 uu/s). A larger implied speed between two samples means a teleport —
 * respawn, goal reset, kickoff — which must snap rather than slide.
 */
const MAX_PLAUSIBLE_SPEED_UU_S = 7000;

/**
 * Frames are normally ~0.12s apart. Replays omit network data during goal
 * celebrations/replays, leaving multi-second gaps that end at kickoff. Never
 * interpolate across a gap this long, and playback skips straight across it.
 */
export const MAX_FRAME_GAP_S = 0.5;

function lerpPosition(a: Position, b: Position, t: number, dt: number): Position {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const dz = b.z - a.z;
  if (Math.sqrt(dx * dx + dy * dy + dz * dz) / dt > MAX_PLAUSIBLE_SPEED_UU_S) {
    return a;
  }
  return { x: a.x + dx * t, y: a.y + dy * t, z: a.z + dz * t };
}

/**
 * Frame state at `time`, linearly interpolating ball and car positions between
 * `frames[index]` (the latest frame at or before `time`) and the next frame.
 * Boost and player presence are discrete and taken from `frames[index]`.
 */
export function interpolateFrame(frames: GameFrame[], index: number, time: number): GameFrame | null {
  const a = frames[index];
  if (!a) return null;
  const b = frames[index + 1];
  if (!b || time <= a.time) return a;

  const dt = b.time - a.time;
  if (dt <= 0 || dt > MAX_FRAME_GAP_S) return a;
  const t = Math.min((time - a.time) / dt, 1);

  const ball = a.ball && b.ball ? lerpPosition(a.ball, b.ball, t, dt) : a.ball;

  const players: PlayerFrame[] = a.players.map((pa) => {
    const pb = b.players.find((p) => p.player_index === pa.player_index);
    if (!pa.position || !pb?.position) return pa;
    return { ...pa, position: lerpPosition(pa.position, pb.position, t, dt) };
  });

  return { ...a, time, ball, players };
}
