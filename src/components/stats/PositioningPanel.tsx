import type { PlayerData } from "../../types/replay";

interface Props {
  player: PlayerData;
}

export function PositioningPanel({ player }: Props) {
  const p = player.stats?.positioning;
  const m = player.stats?.movement;
  if (!p && !m) return null;

  return (
    <div className="bg-[var(--bg-card)] rounded-lg p-4 space-y-4">
      <h4 className="font-semibold text-sm text-[var(--text-secondary)] uppercase tracking-wide">
        Positioning & Movement
      </h4>

      {p && (
        <div className="space-y-2">
          <div className="text-xs text-[var(--text-secondary)] mb-1">
            Field Thirds
          </div>
          <ThirdBar
            def={p.percent_defensive_third ?? 0}
            mid={p.percent_neutral_third ?? 0}
            off={p.percent_offensive_third ?? 0}
          />
          <div className="grid grid-cols-2 gap-3 mt-3">
            <PosStat
              label="Avg Ball Dist"
              value={p.avg_distance_to_ball?.toFixed(0)}
            />
            <PosStat
              label="Avg Mate Dist"
              value={p.avg_distance_to_mates?.toFixed(0)}
            />
            <PosStat
              label="Behind Ball"
              value={
                p.percent_behind_ball != null
                  ? `${p.percent_behind_ball.toFixed(0)}%`
                  : undefined
              }
            />
            <PosStat
              label="Most Back"
              value={
                p.percent_most_back != null
                  ? `${p.percent_most_back.toFixed(0)}%`
                  : undefined
              }
            />
          </div>
        </div>
      )}

      {m && (
        <div className="space-y-2">
          <div className="text-xs text-[var(--text-secondary)] mb-1">
            Movement
          </div>
          <div className="grid grid-cols-2 gap-3">
            <PosStat label="Avg Speed" value={m.avg_speed?.toFixed(0)} />
            <PosStat
              label="Supersonic"
              value={
                m.percent_supersonic_speed != null
                  ? `${m.percent_supersonic_speed.toFixed(0)}%`
                  : undefined
              }
            />
            <PosStat
              label="Ground"
              value={
                m.percent_ground != null
                  ? `${m.percent_ground.toFixed(0)}%`
                  : undefined
              }
            />
            <PosStat
              label="High Air"
              value={
                m.percent_high_air != null
                  ? `${m.percent_high_air.toFixed(0)}%`
                  : undefined
              }
            />
          </div>
        </div>
      )}
    </div>
  );
}

function ThirdBar({
  def,
  mid,
  off,
}: {
  def: number;
  mid: number;
  off: number;
}) {
  return (
    <div className="flex rounded overflow-hidden h-6 text-xs font-medium">
      <div
        className="bg-blue-600 flex items-center justify-center"
        style={{ width: `${def}%` }}
      >
        {def > 10 && `${def.toFixed(0)}%`}
      </div>
      <div
        className="bg-gray-500 flex items-center justify-center"
        style={{ width: `${mid}%` }}
      >
        {mid > 10 && `${mid.toFixed(0)}%`}
      </div>
      <div
        className="bg-orange-500 flex items-center justify-center"
        style={{ width: `${off}%` }}
      >
        {off > 10 && `${off.toFixed(0)}%`}
      </div>
    </div>
  );
}

function PosStat({ label, value }: { label: string; value?: string }) {
  return (
    <div>
      <div className="text-base font-bold">{value ?? "-"}</div>
      <div className="text-xs text-[var(--text-secondary)]">{label}</div>
    </div>
  );
}
