import type { PlayerData } from "../../types/replay";

interface Props {
  player: PlayerData;
  teamColor: "blue" | "orange";
}

export function StatCard({ player, teamColor }: Props) {
  const core = player.stats?.core;
  const demo = player.stats?.demo;
  const borderColor =
    teamColor === "blue" ? "border-blue-500/30" : "border-orange-500/30";
  const nameColor =
    teamColor === "blue" ? "text-blue-400" : "text-orange-400";

  return (
    <div
      className={`bg-[var(--bg-card)] rounded-lg p-4 border ${borderColor}`}
    >
      <div className="flex items-center justify-between mb-3">
        <h3 className={`font-semibold ${nameColor}`}>
          {player.name ?? "Unknown"}
        </h3>
        {player.mvp && (
          <span className="text-xs bg-yellow-500/20 text-yellow-400 px-2 py-0.5 rounded">
            MVP
          </span>
        )}
      </div>
      <div className="grid grid-cols-3 gap-3 text-center">
        <Stat label="Goals" value={core?.goals} />
        <Stat label="Assists" value={core?.assists} />
        <Stat label="Saves" value={core?.saves} />
        <Stat label="Shots" value={core?.shots} />
        <Stat label="Score" value={core?.score} />
        <Stat
          label="Shot %"
          value={
            core?.shooting_percentage != null
              ? `${core.shooting_percentage.toFixed(0)}%`
              : undefined
          }
        />
        <Stat label="Demos" value={demo?.inflicted} />
        <Stat label="Demoed" value={demo?.taken} />
      </div>
    </div>
  );
}

function Stat({
  label,
  value,
}: {
  label: string;
  value?: number | string | null;
}) {
  return (
    <div>
      <div className="text-lg font-bold">{value ?? "-"}</div>
      <div className="text-xs text-[var(--text-secondary)]">{label}</div>
    </div>
  );
}
