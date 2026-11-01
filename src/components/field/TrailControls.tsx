import { Route } from "lucide-react";

interface Props {
  showTrails: boolean;
  setShowTrails: (v: boolean) => void;
  trailSeconds: number;
  setTrailSeconds: (v: number) => void;
}

export function TrailControls({
  showTrails,
  setShowTrails,
  trailSeconds,
  setTrailSeconds,
}: Props) {
  return (
    <div className="bg-[var(--bg-card)] rounded-lg p-3 space-y-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Route className="w-3.5 h-3.5 text-[var(--accent-green)]" />
          <span className="text-xs font-semibold text-[var(--text-secondary)] uppercase tracking-wide">
            Rotation Trails
          </span>
        </div>
        <button
          onClick={() => setShowTrails(!showTrails)}
          className={`px-2.5 py-1 rounded text-xs font-medium transition ${
            showTrails
              ? "bg-[var(--accent-green)] text-white"
              : "text-[var(--text-secondary)] bg-[var(--bg-secondary)]"
          }`}
        >
          {showTrails ? "On" : "Off"}
        </button>
      </div>

      {showTrails && (
        <div className="flex items-center gap-2">
          <span className="text-[10px] text-[var(--text-secondary)]">Trail length:</span>
          {[3, 5, 10, 15].map((s) => (
            <button
              key={s}
              onClick={() => setTrailSeconds(s)}
              className={`px-2 py-0.5 rounded text-[10px] font-medium transition ${
                trailSeconds === s
                  ? "bg-[var(--accent-green)]/20 text-[var(--accent-green)]"
                  : "text-[var(--text-secondary)] hover:bg-[var(--bg-secondary)]"
              }`}
            >
              {s}s
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
