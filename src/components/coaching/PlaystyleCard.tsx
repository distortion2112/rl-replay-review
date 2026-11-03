import type { Axis, PlaystyleProfile } from "../../types/profile";
import { rankName } from "../../lib/coaching";

const AXIS_LABELS: Record<Axis, string> = {
  aggression: "Aggression",
  mechanical: "Mechanical",
  boost_discipline: "Boost discipline",
  team_play: "Team play",
  aerial_tendency: "Aerial",
};

/** Five descriptive axes, each a percentile at the player's rank with its 80% range. */
export function PlaystyleCard({ playstyle }: { playstyle: PlaystyleProfile }) {
  return (
    <div className="bg-[var(--bg-card)] rounded-lg p-4 space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="font-semibold text-sm">Playstyle</h3>
        {playstyle.archetype && (
          <span className="text-xs px-2 py-0.5 rounded bg-[var(--accent-blue)]/20 text-blue-300">
            {playstyle.archetype}
          </span>
        )}
      </div>
      {playstyle.axes.length === 0 ? (
        <p className="text-xs text-[var(--text-secondary)]">{playstyle.archetype_reason}</p>
      ) : (
        <>
          <div className="space-y-2">
            {playstyle.axes.map((a) => (
              <div key={a.axis} className="space-y-0.5">
                <div className="flex justify-between text-xs">
                  <span>{AXIS_LABELS[a.axis]}</span>
                  <span className="text-[var(--text-secondary)] tabular-nums">{Math.round(a.value)}</span>
                </div>
                <div
                  className="relative h-2 bg-[var(--bg-secondary)] rounded-full"
                  role="meter"
                  aria-label={AXIS_LABELS[a.axis]}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={Math.round(a.value)}
                >
                  <div
                    className="absolute inset-y-0 bg-blue-500/30 rounded-full"
                    style={{ left: `${a.interval[0]}%`, width: `${Math.max(a.interval[1] - a.interval[0], 1)}%` }}
                  />
                  <div className="absolute inset-y-[-2px] w-1 bg-blue-400 rounded" style={{ left: `calc(${a.value}% - 2px)` }} />
                </div>
              </div>
            ))}
          </div>
          <p className="text-xs text-[var(--text-secondary)]">
            {playstyle.archetype ? "" : `${playstyle.archetype_reason} `}
            Percentile among {rankName(playstyle.rank_used)} players; the shaded band is the 80% range. This
            describes how you play, not whether it's working.
          </p>
        </>
      )}
    </div>
  );
}
