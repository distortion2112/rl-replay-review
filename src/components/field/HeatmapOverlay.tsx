import { useState, useMemo } from "react";
import { Flame, Users, CircleDot } from "lucide-react";
import { usePlaybackStore } from "../../hooks/usePlayback";
import {
  generateHeatmap,
  generateTeamHeatmap,
  generateBallHeatmap,
  type HeatmapGrid,
} from "../../lib/heatmap";

export type { HeatmapGrid };

type HeatmapMode = "off" | "player" | "blue" | "orange" | "ball";

export function useHeatmapState() {
  const [mode, setMode] = useState<HeatmapMode>("off");
  const [selectedIdx, setSelectedIdx] = useState<number>(0);
  return { mode, setMode, selectedIdx, setSelectedIdx };
}

export function HeatmapControls({
  mode,
  setMode,
  selectedIdx,
  setSelectedIdx,
}: ReturnType<typeof useHeatmapState>) {
  const frameData = usePlaybackStore((s) => s.frameData);

  if (!frameData) return null;

  const modes: { id: HeatmapMode; label: string; icon: typeof Flame }[] = [
    { id: "off", label: "Off", icon: CircleDot },
    { id: "player", label: "Player", icon: Flame },
    { id: "blue", label: "Blue", icon: Users },
    { id: "orange", label: "Orange", icon: Users },
    { id: "ball", label: "Ball", icon: CircleDot },
  ];

  return (
    <div className="bg-[var(--bg-card)] rounded-lg p-3 space-y-2">
      <div className="flex items-center gap-2">
        <Flame className="w-3.5 h-3.5 text-[var(--accent-red)]" />
        <span className="text-xs font-semibold text-[var(--text-secondary)] uppercase tracking-wide">
          Heatmap
        </span>
      </div>

      <div className="flex gap-1">
        {modes.map((m) => (
          <button
            key={m.id}
            onClick={() => setMode(m.id)}
            className={`px-2 py-1 rounded text-xs font-medium transition ${
              mode === m.id
                ? m.id === "blue"
                  ? "bg-blue-600 text-white"
                  : m.id === "orange"
                    ? "bg-orange-600 text-white"
                    : "bg-[var(--accent-blue)] text-white"
                : "text-[var(--text-secondary)] hover:bg-[var(--bg-secondary)]"
            }`}
          >
            {m.label}
          </button>
        ))}
      </div>

      {mode === "player" && (
        <div className="flex flex-wrap gap-1">
          {frameData.players.map((p) => (
            <button
              key={p.index}
              onClick={() => setSelectedIdx(p.index)}
              className={`px-2 py-0.5 rounded text-[10px] font-medium transition ${
                selectedIdx === p.index
                  ? p.team === 0
                    ? "bg-blue-600 text-white"
                    : "bg-orange-600 text-white"
                  : "text-[var(--text-secondary)] hover:bg-[var(--bg-secondary)]"
              }`}
            >
              {p.name}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

/**
 * Render a heatmap onto a canvas context.
 * Call this from the field renderer's draw loop.
 */
export function drawHeatmapOverlay(
  ctx: CanvasRenderingContext2D,
  grid: HeatmapGrid,
  canvasWidth: number,
  canvasHeight: number,
  padding: number
) {
  if (grid.max === 0) return;

  const drawW = canvasWidth - padding * 2;
  const drawH = canvasHeight - padding * 2;
  const cellW = drawW / grid.cols;
  const cellH = drawH / grid.rows;

  for (let y = 0; y < grid.rows; y++) {
    for (let x = 0; x < grid.cols; x++) {
      const val = grid.cells[y * grid.cols + x];
      if (val < grid.max * 0.005) continue; // skip near-zero

      const intensity = val / grid.max;

      // Color: transparent -> blue -> cyan -> yellow -> red
      const [r, g, b, a] = heatColor(intensity);
      ctx.fillStyle = `rgba(${r}, ${g}, ${b}, ${a})`;
      ctx.fillRect(
        padding + x * cellW,
        padding + y * cellH,
        cellW + 0.5, // slight overlap to avoid gaps
        cellH + 0.5
      );
    }
  }
}

function heatColor(t: number): [number, number, number, number] {
  // 5-stop gradient with strong visibility
  // Alpha ramps up quickly and caps high
  const alpha = Math.min(0.15 + t * 0.75, 0.85);

  let r: number, g: number, b: number;
  if (t < 0.2) {
    // Dark blue
    const s = t / 0.2;
    r = 0;
    g = Math.round(s * 40);
    b = Math.round(80 + s * 120);
  } else if (t < 0.4) {
    // Blue -> Cyan
    const s = (t - 0.2) / 0.2;
    r = 0;
    g = Math.round(40 + s * 215);
    b = Math.round(200 + s * 55);
  } else if (t < 0.6) {
    // Cyan -> Green
    const s = (t - 0.4) / 0.2;
    r = Math.round(s * 80);
    g = 255;
    b = Math.round(255 - s * 200);
  } else if (t < 0.8) {
    // Green -> Yellow
    const s = (t - 0.6) / 0.2;
    r = Math.round(80 + s * 175);
    g = Math.round(255 - s * 30);
    b = Math.round(55 - s * 55);
  } else {
    // Yellow -> Red/White hot
    const s = (t - 0.8) / 0.2;
    r = 255;
    g = Math.round(225 - s * 180);
    b = Math.round(s * 80);
  }

  return [r, g, b, alpha];
}

/**
 * Hook to compute the heatmap grid (memoized).
 */
export function useHeatmapGrid(
  mode: HeatmapMode,
  selectedIdx: number
): HeatmapGrid | null {
  const frameData = usePlaybackStore((s) => s.frameData);

  return useMemo(() => {
    if (!frameData || mode === "off") return null;

    switch (mode) {
      case "player":
        return generateHeatmap(frameData, selectedIdx);
      case "blue":
        return generateTeamHeatmap(frameData, 0);
      case "orange":
        return generateTeamHeatmap(frameData, 1);
      case "ball":
        return generateBallHeatmap(frameData);
      default:
        return null;
    }
  }, [frameData, mode, selectedIdx]);
}
