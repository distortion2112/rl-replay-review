import type { FrameData } from "../types/frames";
import { fieldToCanvas } from "./field-math";

// Team colors with multiple shades per player
const BLUE_COLORS = ["#3b82f6", "#60a5fa", "#93c5fd"];
const ORANGE_COLORS = ["#f97316", "#fb923c", "#fdba74"];

/**
 * Draw movement trails for all players over a time window.
 * Shows the path each player took leading up to the current frame.
 */
export function drawTrails(
  ctx: CanvasRenderingContext2D,
  frameData: FrameData,
  currentFrameIndex: number,
  canvasWidth: number,
  canvasHeight: number,
  padding: number,
  trailSeconds: number = 5
) {
  const frames = frameData.frames;
  if (frames.length === 0 || currentFrameIndex < 1) return;

  const currentTime = frames[currentFrameIndex]?.time ?? 0;
  const trailStartTime = currentTime - trailSeconds;

  // Find start frame index
  let startIdx = currentFrameIndex;
  while (startIdx > 0 && frames[startIdx].time > trailStartTime) {
    startIdx--;
  }

  // Build path per player
  const paths: Map<number, { x: number; y: number; time: number }[]> = new Map();

  for (let i = startIdx; i <= currentFrameIndex; i++) {
    const frame = frames[i];
    for (const pf of frame.players) {
      if (!pf.position) continue;
      // Only ground-level trail points (skip when on wall/ceiling)
      if (pf.position.z > 500) continue;

      const canvasPos = fieldToCanvas(
        pf.position.x, pf.position.y,
        canvasWidth, canvasHeight, padding
      );

      if (!paths.has(pf.player_index)) {
        paths.set(pf.player_index, []);
      }
      paths.get(pf.player_index)!.push({
        x: canvasPos.x,
        y: canvasPos.y,
        time: frame.time,
      });
    }
  }

  // Draw each player's trail
  for (const [playerIdx, points] of paths) {
    if (points.length < 2) continue;

    const player = frameData.players.find((p) => p.index === playerIdx);
    if (!player) continue;

    // Pick color based on team + player order within team
    const teamPlayers = frameData.players.filter((p) => p.team === player.team);
    const orderInTeam = teamPlayers.findIndex((p) => p.index === playerIdx);
    const palette = player.team === 0 ? BLUE_COLORS : ORANGE_COLORS;
    const color = palette[orderInTeam % palette.length];

    ctx.lineCap = "round";
    ctx.lineJoin = "round";

    // Draw trail as segments with fading opacity (older = more transparent)
    for (let i = 1; i < points.length; i++) {
      const prev = points[i - 1];
      const curr = points[i];

      // Age: 0 = newest, 1 = oldest
      const age = 1 - (curr.time - trailStartTime) / trailSeconds;
      const alpha = Math.max(0.05, (1 - age) * 0.6);

      ctx.beginPath();
      ctx.moveTo(prev.x, prev.y);
      ctx.lineTo(curr.x, curr.y);
      ctx.strokeStyle = hexWithAlpha(color, alpha);
      ctx.lineWidth = 2.5 * (1 - age * 0.5); // thinner for older parts
      ctx.stroke();
    }

    // Draw a small direction arrow at the trail head
    if (points.length >= 2) {
      const last = points[points.length - 1];
      const prev = points[points.length - 2];
      const dx = last.x - prev.x;
      const dy = last.y - prev.y;
      const len = Math.sqrt(dx * dx + dy * dy);
      if (len > 2) {
        const nx = dx / len;
        const ny = dy / len;
        const arrowSize = 4;
        ctx.beginPath();
        ctx.moveTo(last.x, last.y);
        ctx.lineTo(last.x - nx * arrowSize - ny * arrowSize * 0.5, last.y - ny * arrowSize + nx * arrowSize * 0.5);
        ctx.moveTo(last.x, last.y);
        ctx.lineTo(last.x - nx * arrowSize + ny * arrowSize * 0.5, last.y - ny * arrowSize - nx * arrowSize * 0.5);
        ctx.strokeStyle = hexWithAlpha(color, 0.8);
        ctx.lineWidth = 2;
        ctx.stroke();
      }
    }
  }
}

function hexWithAlpha(hex: string, alpha: number): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}
