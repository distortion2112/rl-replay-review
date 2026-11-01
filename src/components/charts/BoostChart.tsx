import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from "recharts";
import type { PlayerData } from "../../types/replay";

interface Props {
  player: PlayerData;
}

export function BoostChart({ player }: Props) {
  const b = player.stats?.boost;
  if (!b) return null;

  const distribution = [
    { range: "0-25%", value: b.percent_boost_0_25 ?? 0 },
    { range: "25-50%", value: b.percent_boost_25_50 ?? 0 },
    { range: "50-75%", value: b.percent_boost_50_75 ?? 0 },
    { range: "75-100%", value: b.percent_boost_75_100 ?? 0 },
  ];

  const colors = ["#ef4444", "#eab308", "#3b82f6", "#22c55e"];

  return (
    <div className="bg-[var(--bg-card)] rounded-lg p-4">
      <h4 className="font-semibold text-sm text-[var(--text-secondary)] uppercase tracking-wide mb-3">
        Boost Distribution
      </h4>
      <ResponsiveContainer width="100%" height={160}>
        <BarChart data={distribution} barCategoryGap="20%">
          <XAxis
            dataKey="range"
            tick={{ fill: "#8b92a8", fontSize: 11 }}
            axisLine={false}
            tickLine={false}
          />
          <YAxis
            tick={{ fill: "#8b92a8", fontSize: 11 }}
            axisLine={false}
            tickLine={false}
            tickFormatter={(v) => `${v}%`}
            width={40}
          />
          <Tooltip
            contentStyle={{
              background: "#22263a",
              border: "1px solid #3a3f52",
              borderRadius: 8,
              color: "#e4e8f1",
              fontSize: 12,
            }}
            formatter={(value) => [`${Number(value).toFixed(1)}%`, "Time"]}
          />
          <Bar dataKey="value" radius={[4, 4, 0, 0]}>
            {distribution.map((_, i) => (
              <Cell key={i} fill={colors[i]} fillOpacity={0.8} />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}
