import { useRef, useEffect, useState, useCallback } from "react";
import { FIELD, BOOST_PADS, fieldToCanvas } from "../../lib/field-math";
import { usePlaybackStore } from "../../hooks/usePlayback";
import type { GameFrame, FramePlayer } from "../../types/frames";

export function LiveFieldView() {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [size, setSize] = useState({ width: 400, height: 500 });

  const { frameData } = usePlaybackStore();
  const currentFrame = usePlaybackStore((s) => s.currentFrame());

  const updateSize = useCallback(() => {
    const container = containerRef.current;
    if (!container) return;
    const w = container.clientWidth;
    const h = Math.round(w * (FIELD.LENGTH / FIELD.WIDTH));
    setSize({ width: w, height: h });
  }, []);

  useEffect(() => {
    updateSize();
    const observer = new ResizeObserver(updateSize);
    if (containerRef.current) observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, [updateSize]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const { width, height } = size;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);

    // Background
    ctx.fillStyle = "#1a1d27";
    ctx.fillRect(0, 0, width, height);

    const pad = 20;

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
      ctx.strokeRect(gl.x, gl.y, gr.x - gl.x, depth);
    }

    // Boost pads
    for (const bp of BOOST_PADS) {
      const pos = fieldToCanvas(bp.x, bp.y, width, height, pad);
      ctx.beginPath();
      const r = bp.big ? Math.max(4, drawW * 0.012) : Math.max(2, drawW * 0.006);
      ctx.arc(pos.x, pos.y, r, 0, Math.PI * 2);
      ctx.fillStyle = bp.big ? "#eab30866" : "#6b728044";
      ctx.fill();
    }

    // Draw current frame data
    if (currentFrame && frameData) {
      drawFrame(ctx, currentFrame, frameData.players, width, height, pad);
    }

    // Goal labels
    ctx.font = "bold 10px Inter, sans-serif";
    ctx.textAlign = "center";
    const blueGoalY = fieldToCanvas(0, -FIELD.HALF_LENGTH, width, height, pad);
    ctx.fillStyle = "#3b82f680";
    ctx.fillText("BLUE", blueGoalY.x, blueGoalY.y + 14);
    const orangeGoalY = fieldToCanvas(0, FIELD.HALF_LENGTH, width, height, pad);
    ctx.fillStyle = "#f9731680";
    ctx.fillText("ORANGE", orangeGoalY.x, orangeGoalY.y - 6);
  }, [currentFrame, frameData, size]);

  return (
    <div ref={containerRef} className="w-full">
      <canvas
        ref={canvasRef}
        style={{ width: size.width, height: size.height }}
        className="rounded-lg"
      />
    </div>
  );
}

function drawFrame(
  ctx: CanvasRenderingContext2D,
  frame: GameFrame,
  players: FramePlayer[],
  w: number,
  h: number,
  pad: number
) {
  // Draw ball
  if (frame.ball) {
    const pos = fieldToCanvas(frame.ball.x, frame.ball.y, w, h, pad);
    ctx.beginPath();
    ctx.arc(pos.x, pos.y, 8, 0, Math.PI * 2);
    ctx.fillStyle = "#ffffff";
    ctx.fill();
    ctx.strokeStyle = "#ffffff44";
    ctx.lineWidth = 2;
    ctx.stroke();
  }

  // Draw players
  for (const pf of frame.players) {
    if (!pf.position) continue;

    const pos = fieldToCanvas(pf.position.x, pf.position.y, w, h, pad);

    // Try to find player info
    const playerInfo = players.find((p) => p.index === pf.player_index);
    const isBlue = playerInfo ? playerInfo.team === 0 : true;
    const color = isBlue ? "#3b82f6" : "#f97316";

    // Player dot
    ctx.beginPath();
    ctx.arc(pos.x, pos.y, 6, 0, Math.PI * 2);
    ctx.fillStyle = color;
    ctx.fill();

    // Boost indicator ring
    if (pf.boost_amount != null) {
      const boostPct = pf.boost_amount / 255;
      ctx.beginPath();
      ctx.arc(pos.x, pos.y, 9, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * boostPct);
      ctx.strokeStyle = "#eab308";
      ctx.lineWidth = 2;
      ctx.stroke();
    }

    // Player name label
    if (playerInfo) {
      ctx.fillStyle = color;
      ctx.font = "9px Inter, sans-serif";
      ctx.textAlign = "center";
      ctx.fillText(playerInfo.name, pos.x, pos.y - 12);
    }
  }
}
