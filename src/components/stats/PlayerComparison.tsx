import type { PlayerData } from "../../types/replay";

interface Props {
  players: PlayerData[];
  teamColors: Map<string, "blue" | "orange">;
}

export function PlayerComparison({ players, teamColors }: Props) {
  if (players.length === 0) return null;

  const metrics = [
    {
      label: "Score",
      getValue: (p: PlayerData) => p.stats?.core?.score ?? 0,
    },
    {
      label: "Goals",
      getValue: (p: PlayerData) => p.stats?.core?.goals ?? 0,
    },
    {
      label: "Assists",
      getValue: (p: PlayerData) => p.stats?.core?.assists ?? 0,
    },
    {
      label: "Saves",
      getValue: (p: PlayerData) => p.stats?.core?.saves ?? 0,
    },
    {
      label: "Shots",
      getValue: (p: PlayerData) => p.stats?.core?.shots ?? 0,
    },
    {
      label: "Avg Speed",
      getValue: (p: PlayerData) => p.stats?.movement?.avg_speed ?? 0,
    },
    {
      label: "Avg Boost",
      getValue: (p: PlayerData) => p.stats?.boost?.avg_amount ?? 0,
    },
    { label: "Demos", getValue: (p: PlayerData) => p.stats?.demo?.inflicted ?? 0 },
  ];

  return (
    <div className="bg-[var(--bg-card)] rounded-lg p-4 space-y-3">
      <h4 className="font-semibold text-sm text-[var(--text-secondary)] uppercase tracking-wide">
        Player Comparison
      </h4>
      {metrics.map((metric) => {
        const values = players.map((p) => metric.getValue(p));
        const maxVal = Math.max(...values, 1);
        return (
          <div key={metric.label} className="space-y-1">
            <div className="text-xs text-[var(--text-secondary)]">
              {metric.label}
            </div>
            {players.map((p, i) => {
              const val = values[i];
              const pct = (val / maxVal) * 100;
              const team = teamColors.get(p.name ?? "") ?? "blue";
              const barColor =
                team === "blue" ? "bg-blue-500" : "bg-orange-500";
              return (
                <div key={p.name ?? i} className="flex items-center gap-2">
                  <span className="text-xs w-24 truncate text-right">
                    {p.name ?? "?"}
                  </span>
                  <div className="flex-1 bg-[var(--bg-secondary)] rounded-full h-4 overflow-hidden">
                    <div
                      className={`${barColor} h-full rounded-full transition-all`}
                      style={{ width: `${pct}%` }}
                    />
                  </div>
                  <span className="text-xs w-10 text-right font-mono">
                    {val % 1 === 0 ? val : val.toFixed(1)}
                  </span>
                </div>
              );
            })}
          </div>
        );
      })}
    </div>
  );
}
