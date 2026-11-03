import type { ReportItem } from "../../types/coaching";

function tone(item: ReportItem): { border: string; badge: string; label: string } {
  if (item.title.startsWith("Unusual for your rank")) {
    return { border: "border-gray-600", badge: "bg-gray-700 text-gray-300", label: "Style" };
  }
  switch (item.kind) {
    case "strength":
      return { border: "border-[var(--accent-green)]", badge: "bg-green-600/20 text-green-400", label: "Strength" };
    case "weakness":
      return item.top
        ? { border: "border-[var(--accent-red)]", badge: "bg-red-600/20 text-red-400", label: "Top gap" }
        : { border: "border-[var(--accent-orange)]", badge: "bg-orange-600/20 text-orange-400", label: "Gap" };
    default:
      return { border: "border-gray-700", badge: "bg-gray-700/60 text-gray-400", label: "Unclear" };
  }
}

export function FindingCard({ item }: { item: ReportItem }) {
  const t = tone(item);
  return (
    <div className={`bg-[var(--bg-card)] rounded-lg p-3 border-l-4 ${t.border} space-y-1`}>
      <div className="flex items-start justify-between gap-2">
        <h4 className="text-sm font-semibold">{item.title}</h4>
        <span className={`shrink-0 text-[10px] uppercase tracking-wide px-1.5 py-0.5 rounded ${t.badge}`}>
          {t.label}
        </span>
      </div>
      <p className="text-xs text-[var(--text-secondary)] leading-relaxed">{item.explanation}</p>
      {item.action && (
        <p className="text-xs text-[var(--text-primary)] bg-[var(--bg-secondary)] rounded px-2 py-1.5">
          <span className="font-semibold">Practice: </span>
          {item.action}
        </p>
      )}
    </div>
  );
}
