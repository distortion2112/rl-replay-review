import type { FrameData } from "../types/frames";
import { FIELD } from "./field-math";

export interface HeatmapGrid {
  cells: Float32Array;
  cols: number;
  rows: number;
  max: number;
  playerName: string;
}

const GRID_COLS = 80;
const GRID_ROWS = 100;

/**
 * Generate a density heatmap from frame positional data for a specific player.
 * Accumulates all frame positions into a 2D grid and applies Gaussian blur.
 */
export function generateHeatmap(
  frameData: FrameData,
  playerIndex: number
): HeatmapGrid {
  const cols = GRID_COLS;
  const rows = GRID_ROWS;
  const cells = new Float32Array(cols * rows);

  const player = frameData.players.find((p) => p.index === playerIndex);
  const playerName = player?.name ?? "Unknown";

  // Accumulate positions
  for (const frame of frameData.frames) {
    const pf = frame.players.find((p) => p.player_index === playerIndex);
    if (!pf?.position) continue;

    // Only count ground-level positions (z < 300) for meaningful heatmap
    if (pf.position.z > 300) continue;

    const gx = Math.floor(
      ((pf.position.x + FIELD.HALF_WIDTH) / FIELD.WIDTH) * cols
    );
    const gy = Math.floor(
      ((pf.position.y + FIELD.HALF_LENGTH) / FIELD.LENGTH) * rows
    );

    if (gx >= 0 && gx < cols && gy >= 0 && gy < rows) {
      cells[gy * cols + gx] += 1;
    }
  }

  // Gaussian blur (3x3 kernel, 2 passes for smoothness)
  for (let pass = 0; pass < 2; pass++) {
    gaussianBlur(cells, cols, rows);
  }

  // Find max
  let max = 0;
  for (let i = 0; i < cells.length; i++) {
    if (cells[i] > max) max = cells[i];
  }

  return { cells, cols, rows, max, playerName };
}

/**
 * Generate a combined team heatmap (all players on a team).
 */
export function generateTeamHeatmap(
  frameData: FrameData,
  team: number
): HeatmapGrid {
  const cols = GRID_COLS;
  const rows = GRID_ROWS;
  const cells = new Float32Array(cols * rows);

  const teamPlayers = frameData.players.filter((p) => p.team === team);
  const teamName = team === 0 ? "Blue Team" : "Orange Team";

  for (const frame of frameData.frames) {
    for (const pf of frame.players) {
      if (!pf.position) continue;
      if (pf.position.z > 300) continue;

      const player = teamPlayers.find((p) => p.index === pf.player_index);
      if (!player) continue;

      const gx = Math.floor(
        ((pf.position.x + FIELD.HALF_WIDTH) / FIELD.WIDTH) * cols
      );
      const gy = Math.floor(
        ((pf.position.y + FIELD.HALF_LENGTH) / FIELD.LENGTH) * rows
      );

      if (gx >= 0 && gx < cols && gy >= 0 && gy < rows) {
        cells[gy * cols + gx] += 1;
      }
    }
  }

  for (let pass = 0; pass < 2; pass++) {
    gaussianBlur(cells, cols, rows);
  }

  let max = 0;
  for (let i = 0; i < cells.length; i++) {
    if (cells[i] > max) max = cells[i];
  }

  return { cells, cols, rows, max, playerName: teamName };
}

/**
 * Generate ball heatmap showing where the ball spends time.
 */
export function generateBallHeatmap(frameData: FrameData): HeatmapGrid {
  const cols = GRID_COLS;
  const rows = GRID_ROWS;
  const cells = new Float32Array(cols * rows);

  for (const frame of frameData.frames) {
    if (!frame.ball) continue;

    const gx = Math.floor(
      ((frame.ball.x + FIELD.HALF_WIDTH) / FIELD.WIDTH) * cols
    );
    const gy = Math.floor(
      ((frame.ball.y + FIELD.HALF_LENGTH) / FIELD.LENGTH) * rows
    );

    if (gx >= 0 && gx < cols && gy >= 0 && gy < rows) {
      cells[gy * cols + gx] += 1;
    }
  }

  for (let pass = 0; pass < 2; pass++) {
    gaussianBlur(cells, cols, rows);
  }

  let max = 0;
  for (let i = 0; i < cells.length; i++) {
    if (cells[i] > max) max = cells[i];
  }

  return { cells, cols, rows, max, playerName: "Ball" };
}

function gaussianBlur(cells: Float32Array, cols: number, rows: number) {
  const temp = new Float32Array(cells.length);
  // 3x3 Gaussian kernel: [1,2,1; 2,4,2; 1,2,1] / 16
  const k = [1, 2, 1, 2, 4, 2, 1, 2, 1];
  const kSum = 16;

  for (let y = 1; y < rows - 1; y++) {
    for (let x = 1; x < cols - 1; x++) {
      let sum = 0;
      for (let ky = -1; ky <= 1; ky++) {
        for (let kx = -1; kx <= 1; kx++) {
          sum += cells[(y + ky) * cols + (x + kx)] * k[(ky + 1) * 3 + (kx + 1)];
        }
      }
      temp[y * cols + x] = sum / kSum;
    }
  }

  cells.set(temp);
}
