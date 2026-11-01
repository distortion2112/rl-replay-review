import { AlertTriangle, CheckCircle, Fuel, RotateCcw } from "lucide-react";
import type { RotationInsight, BoostInsight } from "../../types/analysis";

interface Props {
  rotationInsights: RotationInsight[];
  boostInsights: BoostInsight[];
}

export function AnalysisPanel({ rotationInsights, boostInsights }: Props) {
  return (
    <div className="space-y-4">
      <div className="bg-[var(--bg-card)] rounded-lg p-4 space-y-3">
        <div className="flex items-center gap-2">
          <RotateCcw className="w-4 h-4 text-[var(--accent-blue)]" />
          <h4 className="font-semibold text-sm text-[var(--text-secondary)] uppercase tracking-wide">
            Rotation Analysis
          </h4>
        </div>
        {rotationInsights.map((r) => (
          <InsightRow
            key={r.player_name}
            severity={r.severity}
            text={r.explanation}
          />
        ))}
      </div>

      <div className="bg-[var(--bg-card)] rounded-lg p-4 space-y-3">
        <div className="flex items-center gap-2">
          <Fuel className="w-4 h-4 text-[var(--accent-orange)]" />
          <h4 className="font-semibold text-sm text-[var(--text-secondary)] uppercase tracking-wide">
            Boost Efficiency
          </h4>
        </div>
        {boostInsights.map((b) => (
          <div key={b.player_name} className="space-y-1">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">{b.player_name}</span>
              <span className="text-sm font-bold">
                {b.efficiency_score.toFixed(0)}/100
              </span>
            </div>
            <div className="bg-[var(--bg-secondary)] rounded-full h-2 overflow-hidden">
              <div
                className={`h-full rounded-full ${ratingColor(b.rating)}`}
                style={{ width: `${b.efficiency_score}%` }}
              />
            </div>
            <p className="text-xs text-[var(--text-secondary)]">
              {b.explanation}
            </p>
          </div>
        ))}
      </div>
    </div>
  );
}

function InsightRow({
  severity,
  text,
}: {
  severity: string;
  text: string;
}) {
  const Icon = severity === "none" ? CheckCircle : AlertTriangle;
  const iconColor =
    severity === "high"
      ? "text-[var(--accent-red)]"
      : severity === "moderate"
        ? "text-[var(--accent-yellow)]"
        : "text-[var(--accent-green)]";

  return (
    <div className="flex gap-2">
      <Icon className={`w-4 h-4 mt-0.5 shrink-0 ${iconColor}`} />
      <p className="text-sm text-[var(--text-secondary)]">{text}</p>
    </div>
  );
}

function ratingColor(rating: string): string {
  switch (rating) {
    case "excellent":
      return "bg-[var(--accent-green)]";
    case "good":
      return "bg-[var(--accent-blue)]";
    case "needs_work":
      return "bg-[var(--accent-yellow)]";
    case "poor":
      return "bg-[var(--accent-red)]";
    default:
      return "bg-gray-500";
  }
}
