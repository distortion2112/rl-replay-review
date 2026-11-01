import type { PlayerData } from "../../types/replay";

interface Props {
  player: PlayerData;
}

export function BoostPanel({ player }: Props) {
  const b = player.stats?.boost;
  if (!b) return null;

  const totalPads =
    (b.count_collected_small ?? 0) + (b.count_collected_big ?? 0);
  const smallRatio =
    totalPads > 0 ? ((b.count_collected_small ?? 0) / totalPads) * 100 : 0;
  const overfillPct =
    b.amount_collected && b.amount_collected > 0
      ? ((b.amount_overfill ?? 0) / b.amount_collected) * 100
      : 0;

  return (
    <div className="bg-[var(--bg-card)] rounded-lg p-4 space-y-3">
      <h4 className="font-semibold text-sm text-[var(--text-secondary)] uppercase tracking-wide">
        Boost
      </h4>
      <div className="grid grid-cols-2 gap-3">
        <BoostStat label="Avg Amount" value={b.avg_amount?.toFixed(0)} />
        <BoostStat label="BPM" value={b.bpm?.toFixed(0)} />
        <BoostStat label="BCPM" value={b.bcpm?.toFixed(0)} />
        <BoostStat
          label="Zero Boost"
          value={
            b.percent_zero_boost != null
              ? `${b.percent_zero_boost.toFixed(1)}%`
              : undefined
          }
          warn={b.percent_zero_boost != null && b.percent_zero_boost > 30}
        />
        <BoostStat
          label="Small Pad %"
          value={`${smallRatio.toFixed(0)}%`}
          good={smallRatio > 60}
        />
        <BoostStat
          label="Overfill"
          value={`${overfillPct.toFixed(0)}%`}
          warn={overfillPct > 25}
        />
        <BoostStat label="Stolen" value={b.amount_stolen?.toFixed(0)} />
        <BoostStat
          label="Big Pads"
          value={b.count_collected_big?.toString()}
        />
      </div>
    </div>
  );
}

function BoostStat({
  label,
  value,
  warn,
  good,
}: {
  label: string;
  value?: string;
  warn?: boolean;
  good?: boolean;
}) {
  const color = warn
    ? "text-[var(--accent-red)]"
    : good
      ? "text-[var(--accent-green)]"
      : "text-[var(--text-primary)]";
  return (
    <div>
      <div className={`text-base font-bold ${color}`}>{value ?? "-"}</div>
      <div className="text-xs text-[var(--text-secondary)]">{label}</div>
    </div>
  );
}
