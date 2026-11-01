import {
  PieChart,
  Pie,
  Cell,
  ResponsiveContainer,
  Legend,
  Tooltip,
} from "recharts";
import type { PlayerData } from "../../types/replay";

interface Props {
  player: PlayerData;
}

export function MovementChart({ player }: Props) {
  const m = player.stats?.movement;
  if (!m) return null;

  const speedData = [
    { name: "Slow", value: m.percent_slow_speed ?? 0, color: "#ef4444" },
    { name: "Boost", value: m.percent_boost_speed ?? 0, color: "#3b82f6" },
    { name: "Supersonic", value: m.percent_supersonic_speed ?? 0, color: "#22c55e" },
  ].filter((d) => d.value > 0);

  const airData = [
    { name: "Ground", value: m.percent_ground ?? 0, color: "#6b7280" },
    { name: "Low Air", value: m.percent_low_air ?? 0, color: "#3b82f6" },
    { name: "High Air", value: m.percent_high_air ?? 0, color: "#a855f7" },
  ].filter((d) => d.value > 0);

  return (
    <div className="bg-[var(--bg-card)] rounded-lg p-4">
      <h4 className="font-semibold text-sm text-[var(--text-secondary)] uppercase tracking-wide mb-3">
        Movement Breakdown
      </h4>
      <div className="grid grid-cols-2 gap-2">
        <div>
          <div className="text-xs text-center text-[var(--text-secondary)] mb-1">Speed</div>
          <ResponsiveContainer width="100%" height={120}>
            <PieChart>
              <Pie
                data={speedData}
                dataKey="value"
                cx="50%"
                cy="50%"
                innerRadius={25}
                outerRadius={45}
                paddingAngle={2}
              >
                {speedData.map((d, i) => (
                  <Cell key={i} fill={d.color} />
                ))}
              </Pie>
              <Tooltip
                contentStyle={{
                  background: "#22263a",
                  border: "1px solid #3a3f52",
                  borderRadius: 8,
                  color: "#e4e8f1",
                  fontSize: 11,
                }}
                formatter={(value) => `${Number(value).toFixed(1)}%`}
              />
              <Legend
                iconSize={8}
                wrapperStyle={{ fontSize: 10 }}
                formatter={(value) => <span style={{ color: "#8b92a8" }}>{value}</span>}
              />
            </PieChart>
          </ResponsiveContainer>
        </div>
        <div>
          <div className="text-xs text-center text-[var(--text-secondary)] mb-1">Altitude</div>
          <ResponsiveContainer width="100%" height={120}>
            <PieChart>
              <Pie
                data={airData}
                dataKey="value"
                cx="50%"
                cy="50%"
                innerRadius={25}
                outerRadius={45}
                paddingAngle={2}
              >
                {airData.map((d, i) => (
                  <Cell key={i} fill={d.color} />
                ))}
              </Pie>
              <Tooltip
                contentStyle={{
                  background: "#22263a",
                  border: "1px solid #3a3f52",
                  borderRadius: 8,
                  color: "#e4e8f1",
                  fontSize: 11,
                }}
                formatter={(value) => `${Number(value).toFixed(1)}%`}
              />
              <Legend
                iconSize={8}
                wrapperStyle={{ fontSize: 10 }}
                formatter={(value) => <span style={{ color: "#8b92a8" }}>{value}</span>}
              />
            </PieChart>
          </ResponsiveContainer>
        </div>
      </div>
    </div>
  );
}
