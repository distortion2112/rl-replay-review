import { useState } from "react";
import { GraduationCap, Loader2, Pencil, RefreshCw } from "lucide-react";
import { useCoachingStore } from "../../hooks/useCoaching";
import { FINDING_GROUPS, GROUP_LABELS, findingGroup, rankName, type FindingGroup } from "../../lib/coaching";
import { FindingCard } from "./FindingCard";
import { PlaystyleCard } from "./PlaystyleCard";
import { ProfileForm } from "./ProfileForm";

const STATUS_TEXT = {
  searching: "Finding your recent ranked 2v2 games…",
  aggregating: "Reading your games",
  analyzing: "Comparing you with your rank…",
} as const;

export function CoachingView() {
  const { profile, status, progress, error, report, pack, playstyle, aggregate, generate } = useCoachingStore();
  const [editing, setEditing] = useState(false);
  const [group, setGroup] = useState<FindingGroup | "all">("all");

  if (!profile || editing) {
    return (
      <div className="h-full overflow-y-auto p-6 flex justify-center">
        <ProfileForm onDone={() => setEditing(false)} />
      </div>
    );
  }

  const busy = status === "searching" || status === "aggregating" || status === "analyzing";
  const top = report?.items.filter((i) => i.top) ?? [];
  const rest = (report?.items.filter((i) => !i.top) ?? []).filter(
    (i) => group === "all" || findingGroup(i.finding_id) === group,
  );
  const substituted =
    pack && (pack.current_tier_used !== pack.current_tier || pack.target_tier_used !== pack.target_tier);

  return (
    <div className="h-full overflow-y-auto px-4 py-4">
      <div className="max-w-[1200px] mx-auto space-y-4">
        {/* Header */}
        <div className="flex items-center gap-3 flex-wrap">
          <GraduationCap className="w-5 h-5 text-[var(--accent-blue)]" />
          <div>
            <div className="font-semibold">{profile.name ?? profile.playerId}</div>
            <div className="text-xs text-[var(--text-secondary)]">
              {rankName(profile.rank)} · ranked 2v2 · last {profile.games} games
              {playstyle?.archetype ? ` · ${playstyle.archetype}` : ""}
            </div>
          </div>
          <div className="flex-1" />
          <button
            onClick={() => setEditing(true)}
            disabled={busy}
            className="px-3 py-1.5 rounded text-xs bg-[var(--bg-card)] hover:bg-gray-700 flex items-center gap-1 disabled:opacity-40"
          >
            <Pencil className="w-3 h-3" /> Change
          </button>
          <button
            onClick={() => generate()}
            disabled={busy}
            className="px-3 py-1.5 rounded text-xs bg-[var(--accent-blue)] text-white flex items-center gap-1 disabled:opacity-40"
          >
            {busy ? <Loader2 className="w-3 h-3 animate-spin" /> : <RefreshCw className="w-3 h-3" />}
            {report ? "Refresh report" : "Generate report"}
          </button>
        </div>

        {/* Status */}
        {busy && (
          <div className="bg-[var(--bg-card)] rounded-lg p-3 text-sm text-[var(--text-secondary)] flex items-center gap-2" role="status">
            <Loader2 className="w-4 h-4 animate-spin" />
            {STATUS_TEXT[status]}
            {status === "aggregating" && progress && ` (${progress.current} of ${progress.total})`}
          </div>
        )}
        {status === "error" && error && (
          <div className="bg-red-950/40 border border-red-900 rounded-lg p-3 text-sm text-red-300" role="alert">
            {error}
          </div>
        )}
        {!report && !busy && status !== "error" && (
          <div className="bg-[var(--bg-card)] rounded-lg p-6 text-center text-sm text-[var(--text-secondary)]">
            Generate a report to compare your recent games with {rankName(profile.rank)} players and the rank above.
          </div>
        )}

        {report && pack && (
          <>
            {/* Summary */}
            <div className="bg-[var(--bg-card)] rounded-lg p-4 space-y-2">
              <p className="text-sm leading-relaxed">{report.summary}</p>
              <p className="text-xs text-[var(--text-secondary)]">
                {pack.games} games analyzed
                {aggregate && aggregate.skipped.length > 0 && `, ${aggregate.skipped.length} skipped (left early or not found)`}
                {" · "}benchmarks from season {pack.benchmark_seasons.join(" and ")}
                {substituted &&
                  ` · no data for ${rankName(pack.current_tier)}${pack.target_tier !== pack.target_tier_used ? ` or ${rankName(pack.target_tier)}` : ""}, using the nearest rank`}
              </p>
            </div>

            <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
              <div className="lg:col-span-2 space-y-4">
                {/* Top findings */}
                {top.length > 0 && (
                  <section className="space-y-2">
                    <h3 className="text-xs font-semibold uppercase tracking-wide text-[var(--text-secondary)]">
                      What matters most
                    </h3>
                    {top.map((i) => (
                      <FindingCard key={i.finding_id} item={i} />
                    ))}
                  </section>
                )}

                {/* Everything else */}
                {report.items.length > top.length && (
                  <section className="space-y-2">
                    <div className="flex items-center gap-2 flex-wrap">
                      <h3 className="text-xs font-semibold uppercase tracking-wide text-[var(--text-secondary)] mr-2">
                        All findings
                      </h3>
                      {(["all", ...FINDING_GROUPS] as const).map((g) => (
                        <button
                          key={g}
                          onClick={() => setGroup(g)}
                          className={`px-2 py-0.5 rounded text-xs ${
                            group === g ? "bg-[var(--accent-blue)] text-white" : "bg-[var(--bg-card)] hover:bg-gray-700"
                          }`}
                        >
                          {g === "all" ? "All" : GROUP_LABELS[g]}
                        </button>
                      ))}
                    </div>
                    {rest.length === 0 ? (
                      <p className="text-xs text-[var(--text-secondary)]">Nothing in this category.</p>
                    ) : (
                      rest.map((i) => <FindingCard key={i.finding_id} item={i} />)
                    )}
                  </section>
                )}
              </div>

              <div className="space-y-4">{playstyle && <PlaystyleCard playstyle={playstyle} />}</div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
