import { useRef, useEffect, useState, useCallback, useMemo } from "react";
import { FIELD, BOOST_PADS, fieldToCanvas } from "../../lib/field-math";
import { usePlaybackStore } from "../../hooks/usePlayback";
import { drawHeatmapOverlay, type HeatmapGrid } from "./HeatmapOverlay";
import { drawTrails } from "../../lib/trails";
import { interpolateFrame } from "../../lib/interpolation";
import type { GameFrame, FramePlayer } from "../../types/frames";

interface LiveFieldProps {
  heatmapGrid?: HeatmapGrid | null;
  showTrails?: boolean;
  trailSeconds?: number;
}

export function LiveFieldView({ heatmapGrid, showTrails = true, trailSeconds = 5 }: LiveFieldProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [size, setSize] = useState({ width: 400, height: 500 });

  const { frameData } = usePlaybackStore();
  const currentFrameIndex = usePlaybackStore((s) => s.currentFrameIndex);
  const playbackTime = usePlaybackStore((s) => s.playbackTime);
  const currentFrame = useMemo(
    () => (frameData ? interpolateFrame(frameData.frames, currentFrameIndex, playbackTime) : null),
    [frameData, currentFrameIndex, playbackTime]
  );

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

    const pad = 24;
    const drawW = width - pad * 2;
    const drawH = height - pad * 2;

    // Background
    ctx.fillStyle = "#0f1117";
    ctx.fillRect(0, 0, width, height);

    // Draw the RL pitch shape with rounded corners and wall ramp shading
    drawPitch(ctx, width, height, pad, drawW, drawH);

    // Heatmap overlay (between pitch and players)
    if (heatmapGrid && heatmapGrid.max > 0) {
      drawHeatmapOverlay(ctx, heatmapGrid, width, height, pad);
    }

    // Rotation trails
    if (showTrails && frameData) {
      drawTrails(ctx, frameData, currentFrameIndex, width, height, pad, trailSeconds);
    }

    // Boost pads (pass current frame for pickup detection)
    drawBoostPads(ctx, width, height, pad, drawW, currentFrame);

    // Draw current frame data
    if (currentFrame && frameData) {
      drawFrame(ctx, currentFrame, frameData.players, width, height, pad, drawW);
    }

    // Goal labels
    ctx.font = "bold 10px Inter, sans-serif";
    ctx.textAlign = "center";
    const blueGoalY = fieldToCanvas(0, -FIELD.HALF_LENGTH, width, height, pad);
    ctx.fillStyle = "#3b82f6aa";
    ctx.fillText("BLUE", blueGoalY.x, blueGoalY.y + 14);
    const orangeGoalY = fieldToCanvas(0, FIELD.HALF_LENGTH, width, height, pad);
    ctx.fillStyle = "#f97316aa";
    ctx.fillText("ORANGE", orangeGoalY.x, orangeGoalY.y - 6);

    // Height legend
    ctx.textAlign = "right";
    ctx.font = "8px Inter, sans-serif";
    ctx.fillStyle = "#ffffff30";
    ctx.fillText("Height:", width - 8, height - 28);
    const legendY = height - 18;
    const legendX = width - 60;
    for (let i = 0; i < 5; i++) {
      const t = i / 4;
      ctx.fillStyle = heightColor(t, 0.8);
      ctx.fillRect(legendX + i * 12, legendY, 10, 8);
    }
    ctx.fillStyle = "#ffffff30";
    ctx.font = "7px Inter, sans-serif";
    ctx.textAlign = "left";
    ctx.fillText("ground", legendX, legendY + 16);
    ctx.textAlign = "right";
    ctx.fillText("ceiling", legendX + 58, legendY + 16);

    // Debug overlay
    if (currentFrame) {
      ctx.fillStyle = "#ffffff66";
      ctx.font = "9px monospace";
      ctx.textAlign = "left";
      let debugY = 12;
      ctx.fillText(
        `t=${currentFrame.time.toFixed(1)}s | ${currentFrame.players.length} players`,
        4, debugY
      );
    }
  }, [currentFrame, currentFrameIndex, frameData, size, heatmapGrid, showTrails, trailSeconds]);

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

/** Map height (0=ground, 1=ceiling) to a color */
function heightColor(t: number, alpha = 1): string {
  // Green (ground) -> Cyan (mid) -> Magenta (ceiling)
  const r = Math.round(t * 200);
  const g = Math.round((1 - t) * 200 + 55);
  const b = Math.round(100 + t * 155);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function drawPitch(
  ctx: CanvasRenderingContext2D,
  w: number, h: number,
  pad: number,
  drawW: number, _drawH: number
) {
  const tl = fieldToCanvas(-FIELD.HALF_WIDTH, -FIELD.HALF_LENGTH, w, h, pad);
  const br = fieldToCanvas(FIELD.HALF_WIDTH, FIELD.HALF_LENGTH, w, h, pad);
  const fw = br.x - tl.x;
  const fh = br.y - tl.y;

  // Corner radius in canvas pixels
  const cr = (FIELD.CORNER_RADIUS / FIELD.WIDTH) * drawW;

  // Wall ramp gradient (darker at edges = wall ramp zone)
  const rampWidth = (400 / FIELD.WIDTH) * drawW; // ~400 UU ramp zone

  // Field floor fill with rounded rect
  ctx.save();
  ctx.beginPath();
  ctx.roundRect(tl.x, tl.y, fw, fh, cr);
  ctx.fillStyle = "#1a2030";
  ctx.fill();

  // Ramp shading along walls - gradient from edge inward
  // Left wall ramp
  const leftGrad = ctx.createLinearGradient(tl.x, 0, tl.x + rampWidth, 0);
  leftGrad.addColorStop(0, "rgba(40, 50, 70, 0.6)");
  leftGrad.addColorStop(1, "rgba(40, 50, 70, 0)");
  ctx.fillStyle = leftGrad;
  ctx.fill();

  // Right wall ramp
  const rightGrad = ctx.createLinearGradient(br.x, 0, br.x - rampWidth, 0);
  rightGrad.addColorStop(0, "rgba(40, 50, 70, 0.6)");
  rightGrad.addColorStop(1, "rgba(40, 50, 70, 0)");
  ctx.fillStyle = rightGrad;
  ctx.fill();

  // Top wall ramp
  const topGrad = ctx.createLinearGradient(0, tl.y, 0, tl.y + rampWidth);
  topGrad.addColorStop(0, "rgba(40, 50, 70, 0.6)");
  topGrad.addColorStop(1, "rgba(40, 50, 70, 0)");
  ctx.fillStyle = topGrad;
  ctx.fill();

  // Bottom wall ramp
  const botGrad = ctx.createLinearGradient(0, br.y, 0, br.y - rampWidth);
  botGrad.addColorStop(0, "rgba(40, 50, 70, 0.6)");
  botGrad.addColorStop(1, "rgba(40, 50, 70, 0)");
  ctx.fillStyle = botGrad;
  ctx.fill();

  ctx.restore();

  // Field outline with rounded corners
  ctx.beginPath();
  ctx.roundRect(tl.x, tl.y, fw, fh, cr);
  ctx.strokeStyle = "#3a4f6a";
  ctx.lineWidth = 1.5;
  ctx.stroke();

  // Corner ramp arcs (show the curved ramp transition in corners)
  ctx.strokeStyle = "#2a3a50";
  ctx.lineWidth = 1;
  const innerCr = cr * 0.5;
  // Top-left corner
  ctx.beginPath();
  ctx.arc(tl.x + cr, tl.y + cr, innerCr, Math.PI, Math.PI * 1.5);
  ctx.stroke();
  // Top-right
  ctx.beginPath();
  ctx.arc(br.x - cr, tl.y + cr, innerCr, Math.PI * 1.5, Math.PI * 2);
  ctx.stroke();
  // Bottom-left
  ctx.beginPath();
  ctx.arc(tl.x + cr, br.y - cr, innerCr, Math.PI * 0.5, Math.PI);
  ctx.stroke();
  // Bottom-right
  ctx.beginPath();
  ctx.arc(br.x - cr, br.y - cr, innerCr, 0, Math.PI * 0.5);
  ctx.stroke();

  // Center line
  const cl = fieldToCanvas(-FIELD.HALF_WIDTH, 0, w, h, pad);
  const clr = fieldToCanvas(FIELD.HALF_WIDTH, 0, w, h, pad);
  ctx.beginPath();
  ctx.moveTo(cl.x, cl.y);
  ctx.lineTo(clr.x, clr.y);
  ctx.strokeStyle = "#2a3a5066";
  ctx.lineWidth = 1;
  ctx.stroke();

  // Center circle
  const center = fieldToCanvas(0, 0, w, h, pad);
  const circleR = (1024 / FIELD.WIDTH) * drawW;
  ctx.beginPath();
  ctx.arc(center.x, center.y, circleR, 0, Math.PI * 2);
  ctx.strokeStyle = "#2a3a5066";
  ctx.stroke();

  // Center dot
  ctx.beginPath();
  ctx.arc(center.x, center.y, 3, 0, Math.PI * 2);
  ctx.fillStyle = "#2a3a5066";
  ctx.fill();

  // Goals
  const goalHalfW = FIELD.GOAL_WIDTH / 2;
  for (const yDir of [-1, 1]) {
    const y = yDir * FIELD.HALF_LENGTH;
    const gl = fieldToCanvas(-goalHalfW, y, w, h, pad);
    const gr = fieldToCanvas(goalHalfW, y, w, h, pad);
    const depth = (FIELD.GOAL_DEPTH / FIELD.LENGTH) * (h - pad * 2) * yDir;

    // Goal area fill
    ctx.fillStyle = yDir === -1 ? "rgba(59,130,246,0.08)" : "rgba(249,115,22,0.08)";
    ctx.fillRect(gl.x, gl.y, gr.x - gl.x, depth);

    // Goal outline
    ctx.strokeStyle = yDir === -1 ? "#3b82f666" : "#f9731666";
    ctx.lineWidth = 2;
    ctx.strokeRect(gl.x, gl.y, gr.x - gl.x, depth);
  }
}

// Track pad cooldowns: key = "x,y", value = time when picked up
const padCooldowns = new Map<string, number>();
const BIG_PAD_COOLDOWN = 10; // seconds
const SMALL_PAD_COOLDOWN = 4;
const PICKUP_RADIUS_BIG = 208; // UU proximity for big pad pickup
const PICKUP_RADIUS_SMALL = 144; // UU proximity for small pad pickup

function drawBoostPads(
  ctx: CanvasRenderingContext2D,
  w: number, h: number,
  pad: number,
  drawW: number,
  frame: GameFrame | null
) {
  // Detect pickups: if any player is close to a pad, mark it as picked up
  if (frame) {
    for (const bp of BOOST_PADS) {
      const key = `${bp.x},${bp.y}`;
      const pickupR = bp.big ? PICKUP_RADIUS_BIG : PICKUP_RADIUS_SMALL;

      for (const pf of frame.players) {
        if (!pf.position) continue;
        const dx = pf.position.x - bp.x;
        const dy = pf.position.y - bp.y;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < pickupR && pf.position.z < 200) {
          padCooldowns.set(key, frame.time);
          break;
        }
      }
    }
  }

  for (const bp of BOOST_PADS) {
    const pos = fieldToCanvas(bp.x, bp.y, w, h, pad);
    const r = bp.big ? Math.max(5, drawW * 0.013) : Math.max(2.5, drawW * 0.006);
    const key = `${bp.x},${bp.y}`;
    const cooldown = bp.big ? BIG_PAD_COOLDOWN : SMALL_PAD_COOLDOWN;

    // Check if pad is available
    const lastPickup = padCooldowns.get(key) ?? -999;
    const timeSincePickup = (frame?.time ?? 999) - lastPickup;
    const available = timeSincePickup >= cooldown;
    const respawnPct = available ? 1 : Math.min(timeSincePickup / cooldown, 1);

    if (bp.big) {
      if (available) {
        // Available: bright golden with glow
        ctx.beginPath();
        ctx.arc(pos.x, pos.y, r + 2, 0, Math.PI * 2);
        ctx.fillStyle = "rgba(234, 179, 8, 0.12)";
        ctx.fill();

        ctx.beginPath();
        ctx.arc(pos.x, pos.y, r, 0, Math.PI * 2);
        ctx.fillStyle = "#eab308aa";
        ctx.fill();
        ctx.strokeStyle = "#eab308";
        ctx.lineWidth = 1.5;
        ctx.stroke();
      } else {
        // On cooldown: dim with respawn progress ring
        ctx.beginPath();
        ctx.arc(pos.x, pos.y, r, 0, Math.PI * 2);
        ctx.fillStyle = "#eab30815";
        ctx.fill();

        // Respawn progress arc
        ctx.beginPath();
        ctx.arc(pos.x, pos.y, r + 1, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * respawnPct);
        ctx.strokeStyle = "#eab30844";
        ctx.lineWidth = 1.5;
        ctx.stroke();
      }
    } else {
      if (available) {
        // Available small pad
        ctx.beginPath();
        ctx.arc(pos.x, pos.y, r, 0, Math.PI * 2);
        ctx.fillStyle = "#eab30866";
        ctx.fill();
      } else {
        // On cooldown
        ctx.beginPath();
        ctx.arc(pos.x, pos.y, r, 0, Math.PI * 2);
        ctx.fillStyle = "#6b728018";
        ctx.fill();
      }
    }
  }
}

function drawFrame(
  ctx: CanvasRenderingContext2D,
  frame: GameFrame,
  players: FramePlayer[],
  w: number, h: number,
  pad: number,
  drawW: number
) {
  const maxZ = FIELD.CEILING_HEIGHT;

  // Draw ball shadow + ball
  if (frame.ball) {
    const pos = fieldToCanvas(frame.ball.x, frame.ball.y, w, h, pad);
    const z = frame.ball.z;
    const heightPct = Math.min(z / maxZ, 1);
    const baseR = Math.max(5, drawW * 0.012);

    // Ground shadow (gets smaller/lighter as ball goes higher)
    if (heightPct > 0.05) {
      const shadowR = baseR * (1 - heightPct * 0.5);
      const shadowAlpha = 0.3 * (1 - heightPct * 0.7);
      ctx.beginPath();
      ctx.arc(pos.x, pos.y, shadowR, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(0, 0, 0, ${shadowAlpha})`;
      ctx.fill();
    }

    // Ball - offset upward slightly when aerial (visual cue)
    const visualOffset = heightPct * 6;
    const ballR = baseR + heightPct * 3; // slightly larger when high

    ctx.beginPath();
    ctx.arc(pos.x, pos.y - visualOffset, ballR, 0, Math.PI * 2);
    ctx.fillStyle = heightColor(heightPct, 0.9);
    ctx.fill();
    ctx.strokeStyle = "#ffffff88";
    ctx.lineWidth = 1.5;
    ctx.stroke();

    // "BALL" label with height
    ctx.font = "8px Inter, sans-serif";
    ctx.textAlign = "center";
    ctx.fillStyle = "#ffffff88";
    if (heightPct > 0.1) {
      ctx.fillText(`${Math.round(z)}uu`, pos.x, pos.y - visualOffset - ballR - 3);
    }
  }

  // Draw players
  for (const pf of frame.players) {
    if (!pf.position) continue;

    const pos = fieldToCanvas(pf.position.x, pf.position.y, w, h, pad);
    const z = pf.position.z;
    const heightPct = Math.min(Math.max(z, 0) / maxZ, 1);
    const isAerial = z > 50; // above normal ground level (~17uu)

    const playerInfo = players.find((p) => p.index === pf.player_index);
    const isBlue = playerInfo ? playerInfo.team === 0 : true;
    const teamColor = isBlue ? "#3b82f6" : "#f97316";
    const baseR = Math.max(5, drawW * 0.011);
    const heightRingR = baseR + 3;
    const boostRingR = baseR + 7;

    // Ground shadow when aerial
    if (isAerial) {
      const shadowR = baseR * (1 - heightPct * 0.4);
      const shadowAlpha = 0.25 * (1 - heightPct * 0.6);
      ctx.beginPath();
      ctx.arc(pos.x, pos.y, shadowR, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(0, 0, 0, ${shadowAlpha})`;
      ctx.fill();
    }

    const visualOffset = isAerial ? heightPct * 6 : 0;
    const cx = pos.x;
    const cy = pos.y - visualOffset;

    // --- Layer 1: Player dot (team color) ---
    ctx.beginPath();
    ctx.arc(cx, cy, baseR, 0, Math.PI * 2);
    ctx.fillStyle = teamColor;
    ctx.fill();

    // --- Layer 2: Height ring (thin, always visible, color = height) ---
    ctx.beginPath();
    ctx.arc(cx, cy, heightRingR, 0, Math.PI * 2);
    ctx.strokeStyle = heightColor(heightPct, isAerial ? 0.9 : 0.3);
    ctx.lineWidth = 1.5;
    ctx.stroke();

    // Height text (only when meaningfully aerial)
    if (isAerial) {
      ctx.font = "bold 7px Inter, sans-serif";
      ctx.textAlign = "center";
      ctx.fillStyle = heightColor(heightPct, 0.9);
      ctx.fillText(`${Math.round(z)}`, cx, cy + baseR + 12);
    }

    // --- Layer 3: Boost ring (outer, arc showing amount) ---
    // Background track
    ctx.beginPath();
    ctx.arc(cx, cy, boostRingR, 0, Math.PI * 2);
    ctx.strokeStyle = "rgba(234, 179, 8, 0.12)";
    ctx.lineWidth = 2.5;
    ctx.stroke();

    if (pf.boost_amount != null) {
      const boostPct = pf.boost_amount / 255;

      if (boostPct > 0.01) {
        ctx.beginPath();
        ctx.arc(cx, cy, boostRingR, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * boostPct);
        // Yellow when >30%, orange 10-30%, red <10%
        const boostColor = boostPct > 0.3
          ? `rgba(234, 179, 8, ${0.5 + boostPct * 0.5})`
          : boostPct > 0.1
            ? `rgba(249, 115, 22, ${0.5 + boostPct * 0.5})`
            : `rgba(239, 68, 68, 0.8)`;
        ctx.strokeStyle = boostColor;
        ctx.lineWidth = 2.5;
        ctx.stroke();
      }

      // Boost amount text (small, below height text or below dot)
      ctx.font = "7px Inter, sans-serif";
      ctx.textAlign = "center";
      ctx.fillStyle = "#eab30888";
      const boostTextY = isAerial ? cy + baseR + 20 : cy + baseR + 12;
      ctx.fillText(`${Math.round(boostPct * 100)}%`, cx, boostTextY);
    }

    // --- Player name (above everything) ---
    if (playerInfo) {
      ctx.fillStyle = `${teamColor}cc`;
      ctx.font = "bold 9px Inter, sans-serif";
      ctx.textAlign = "center";
      ctx.fillText(playerInfo.name, cx, cy - boostRingR - 3);
    }
  }
}
