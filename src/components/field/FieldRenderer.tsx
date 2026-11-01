import { useRef, useEffect } from "react";
import { FIELD, BOOST_PADS, fieldToCanvas } from "../../lib/field-math";
import type { PositioningStats } from "../../types/replay";

interface Props {
  positioning?: PositioningStats | null;
  playerName?: string;
  teamColor?: "blue" | "orange";
  width?: number;
  height?: number;
}

export function FieldRenderer({
  positioning,
  playerName,
  teamColor = "blue",
  width = 400,
  height = 500,
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);

    // Clear
    ctx.fillStyle = "#1a1d27";
    ctx.fillRect(0, 0, width, height);

    const pad = 20;

    // Draw heatmap zones if positioning data exists
    if (positioning) {
      drawHeatmap(ctx, positioning, width, height, pad);
    }

    // Field outline
    ctx.strokeStyle = "#3a3f52";
    ctx.lineWidth = 2;
    const tl = fieldToCanvas(-FIELD.HALF_WIDTH, -FIELD.HALF_LENGTH, width, height, pad);
    const br = fieldToCanvas(FIELD.HALF_WIDTH, FIELD.HALF_LENGTH, width, height, pad);
    ctx.strokeRect(tl.x, tl.y, br.x - tl.x, br.y - tl.y);

    // Center line
    const cl = fieldToCanvas(-FIELD.HALF_WIDTH, 0, width, height, pad);
    const cr = fieldToCanvas(FIELD.HALF_WIDTH, 0, width, height, pad);
    ctx.beginPath();
    ctx.moveTo(cl.x, cl.y);
    ctx.lineTo(cr.x, cr.y);
    ctx.strokeStyle = "#3a3f52";
    ctx.lineWidth = 1;
    ctx.stroke();

    // Center circle
    const center = fieldToCanvas(0, 0, width, height, pad);
    const drawW = width - pad * 2;
    const circleRadius = (1024 / FIELD.WIDTH) * drawW;
    ctx.beginPath();
    ctx.arc(center.x, center.y, circleRadius, 0, Math.PI * 2);
    ctx.stroke();

    // Goals
    const goalHalfW = FIELD.GOAL_WIDTH / 2;
    for (const yDir of [-1, 1]) {
      const y = yDir * FIELD.HALF_LENGTH;
      const gl = fieldToCanvas(-goalHalfW, y, width, height, pad);
      const gr = fieldToCanvas(goalHalfW, y, width, height, pad);
      const depth = (FIELD.GOAL_DEPTH / FIELD.LENGTH) * (height - pad * 2) * yDir;
      ctx.strokeStyle = yDir === -1 ? "#3b82f6" : "#f97316";
      ctx.lineWidth = 2;
      ctx.strokeRect(
        gl.x,
        gl.y,
        gr.x - gl.x,
        depth
      );
    }

    // Boost pads
    for (const bp of BOOST_PADS) {
      const pos = fieldToCanvas(bp.x, bp.y, width, height, pad);
      ctx.beginPath();
      ctx.arc(pos.x, pos.y, bp.big ? 6 : 3, 0, Math.PI * 2);
      ctx.fillStyle = bp.big ? "#eab308" : "#6b7280";
      ctx.fill();
    }

    // Player name label
    if (playerName) {
      ctx.fillStyle = teamColor === "blue" ? "#3b82f6" : "#f97316";
      ctx.font = "bold 12px Inter, sans-serif";
      ctx.textAlign = "center";
      ctx.fillText(playerName, width / 2, height - 4);
    }
  }, [positioning, playerName, teamColor, width, height]);

  return (
    <canvas
      ref={canvasRef}
      style={{ width, height }}
      className="rounded-lg"
    />
  );
}

function drawHeatmap(
  ctx: CanvasRenderingContext2D,
  pos: PositioningStats,
  w: number,
  h: number,
  pad: number
) {
  const drawW = w - pad * 2;
  const drawH = h - pad * 2;
  const thirdH = drawH / 3;
  const halfW = drawW / 2;

  // Six zones: defensive/neutral/offensive x own-half/opponent-half
  // Approximate using thirds percentages and halves percentages
  const defPct = pos.percent_defensive_third ?? 33;
  const midPct = pos.percent_neutral_third ?? 33;
  const offPct = pos.percent_offensive_third ?? 33;
  const ownHalf = pos.percent_defensive_half ?? 50;
  const oppHalf = pos.percent_offensive_half ?? 50;

  const zones = [
    // [x, y, w, h, intensity]
    // Defensive third (bottom = blue goal side)
    { x: pad, y: pad + thirdH * 2, w: halfW, h: thirdH, pct: defPct * (ownHalf / 100) },
    { x: pad + halfW, y: pad + thirdH * 2, w: halfW, h: thirdH, pct: defPct * (oppHalf / 100) },
    // Neutral third
    { x: pad, y: pad + thirdH, w: halfW, h: thirdH, pct: midPct * 0.5 },
    { x: pad + halfW, y: pad + thirdH, w: halfW, h: thirdH, pct: midPct * 0.5 },
    // Offensive third (top = orange goal side)
    { x: pad, y: pad, w: halfW, h: thirdH, pct: offPct * (oppHalf / 100) },
    { x: pad + halfW, y: pad, w: halfW, h: thirdH, pct: offPct * (ownHalf / 100) },
  ];

  const maxPct = Math.max(...zones.map((z) => z.pct), 1);

  for (const zone of zones) {
    const intensity = zone.pct / maxPct;
    // Blue to red gradient based on intensity
    const r = Math.round(intensity * 239);
    const g = Math.round((1 - intensity) * 50 + 30);
    const b = Math.round((1 - intensity) * 200 + 50);
    ctx.fillStyle = `rgba(${r}, ${g}, ${b}, ${0.15 + intensity * 0.35})`;
    ctx.fillRect(zone.x, zone.y, zone.w, zone.h);
  }
}
