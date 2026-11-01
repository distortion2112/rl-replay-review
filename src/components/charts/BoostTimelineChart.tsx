import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
} from "recharts";
import { usePlaybackStore } from "../../hooks/usePlayback";

export function BoostTimelineChart() {
  const { frameData } = usePlaybackStore();
  if (!frameData || frameData.frames.length === 0) return null;

  // Build time series: sample every 10th frame for performance
  const players = frameData.players;
  const colors = ["#3b82f6", "#60a5fa", "#93c5fd", "#f97316", "#fb923c", "#fdba74"];

  // Build data points
  const data: Record<string, number | string>[] = [];
  for (let i = 0; i < frameData.frames.length; i += 10) {
    const frame = frameData.frames[i];
    const point: Record<string, number | string> = { time: Number(frame.time.toFixed(1)) };
    for (const pf of frame.players) {
      const player = players.find((p) => p.index === pf.player_index);
      if (player && pf.boost_amount != null) {
        point[player.name] = Math.round((pf.boost_amount / 255) * 100);
      }
    }
    data.push(point);
  }

  if (data.length === 0) return null;

  return (
    <div className="bg-[var(--bg-card)] rounded-lg p-4">
      <h4 className="font-semibold text-sm text-[var(--text-secondary)] uppercase tracking-wide mb-3">
        Boost Over Time
      </h4>
      <ResponsiveContainer width="100%" height={200}>
        <LineChart data={data}>
          <XAxis
            dataKey="time"
            tick={{ fill: "#8b92a8", fontSize: 10 }}
            axisLine={false}
            tickLine={false}
            tickFormatter={(v) => {
              const m = Math.floor(v / 60);
              const s = Math.floor(v % 60);
              return `${m}:${String(s).padStart(2, "0")}`;
            }}
          />
          <YAxis
            tick={{ fill: "#8b92a8", fontSize: 10 }}
            axisLine={false}
            tickLine={false}
            domain={[0, 100]}
            tickFormatter={(v) => `${v}%`}
            width={35}
          />
          <Tooltip
            contentStyle={{
              background: "#22263a",
              border: "1px solid #3a3f52",
              borderRadius: 8,
              color: "#e4e8f1",
              fontSize: 11,
            }}
            formatter={(value) => `${value}%`}
            labelFormatter={(label) => {
              const t = Number(label);
              const m = Math.floor(t / 60);
              const s = Math.floor(t % 60);
              return `${m}:${String(s).padStart(2, "0")}`;
            }}
          />
          {/* Goal lines */}
          {frameData.goals.map((g, i) => (
            <ReferenceLine
              key={i}
              x={Number(g.time.toFixed(1))}
              stroke="#eab308"
              strokeDasharray="3 3"
              strokeOpacity={0.5}
            />
          ))}
          {players.map((player, i) => (
            <Line
              key={player.name}
              type="monotone"
              dataKey={player.name}
              stroke={colors[i % colors.length]}
              dot={false}
              strokeWidth={1.5}
              strokeOpacity={0.8}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
