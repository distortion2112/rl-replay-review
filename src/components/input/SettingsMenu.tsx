import { useState, useEffect } from "react";
import { Settings, X, KeyRound, Trash2, Activity } from "lucide-react";
import { useReplayStore } from "../../hooks/useReplay";
import { getRateLimitStatus } from "../../lib/tauri-commands";
import type { ApiTier, EndpointStatus, RateLimitStatus } from "../../types/analysis";

const TIER_NAMES: Record<ApiTier, string> = {
  regular: "Regular",
  gold: "Gold",
  diamond: "Diamond",
  champion: "Champion",
  gc: "GC",
};

const ENDPOINT_NAMES: Record<EndpointStatus["endpoint"], string> = {
  list: "Replay search",
  get: "Replay stats",
  file: "Replay downloads",
};

export function SettingsMenu() {
  const [open, setOpen] = useState(false);
  const [newKey, setNewKey] = useState("");
  const [saving, setSaving] = useState(false);
  const [rateLimit, setRateLimit] = useState<RateLimitStatus | null>(null);
  const { apiKey, saveApiKey } = useReplayStore();

  useEffect(() => {
    if (open) {
      getRateLimitStatus().then(setRateLimit).catch(() => {});
    }
  }, [open]);

  const handleChangeKey = async () => {
    if (!newKey.trim()) return;
    setSaving(true);
    await saveApiKey(newKey.trim());
    setNewKey("");
    setSaving(false);
  };

  const handleClearKey = async () => {
    await saveApiKey("");
    window.location.reload();
  };

  if (!open) {
    return (
      <button
        onClick={() => setOpen(true)}
        className="p-2 rounded-lg text-[var(--text-secondary)] hover:text-[var(--text-primary)]
                   hover:bg-[var(--bg-secondary)] transition"
        title="Settings"
      >
        <Settings className="w-4 h-4" />
      </button>
    );
  }

  const tierName = rateLimit
    ? TIER_NAMES[rateLimit.tier] + (rateLimit.tier_detected ? "" : " (not yet checked)")
    : "";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-[var(--bg-card)] rounded-xl p-6 w-96 shadow-2xl">
        <div className="flex items-center justify-between mb-4">
          <h2 className="font-semibold text-lg">Settings</h2>
          <button
            onClick={() => setOpen(false)}
            className="p-1 rounded hover:bg-[var(--bg-secondary)] transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="space-y-4">
          <div>
            <label className="text-xs text-[var(--text-secondary)] block mb-1">
              <KeyRound className="w-3 h-3 inline mr-1" />
              API Key
            </label>
            <div className="text-sm text-[var(--text-primary)] bg-[var(--bg-secondary)] rounded px-3 py-2 font-mono">
              {apiKey ? `${apiKey.slice(0, 8)}${"*".repeat(20)}` : "Not set"}
            </div>
          </div>

          {rateLimit && (
            <div>
              <label className="text-xs text-[var(--text-secondary)] block mb-1">
                <Activity className="w-3 h-3 inline mr-1" />
                API usage ({tierName} tier)
              </label>
              <div className="bg-[var(--bg-secondary)] rounded px-3 py-2 space-y-2">
                {rateLimit.endpoints.map((e) => {
                  const pct = e.per_hour ? (e.used_last_hour / e.per_hour) * 100 : 0;
                  const color =
                    pct > 80 ? "var(--accent-red)" : pct > 50 ? "var(--accent-orange)" : "var(--accent-green, #22c55e)";
                  return (
                    <div key={e.endpoint}>
                      <div className="flex justify-between text-sm mb-1">
                        <span>{ENDPOINT_NAMES[e.endpoint]}</span>
                        <span className="text-[var(--text-secondary)]">
                          {e.per_hour
                            ? `${e.used_last_hour} / ${e.per_hour} this hour`
                            : `${e.used_last_hour} this hour, no hourly cap`}
                        </span>
                      </div>
                      {e.per_hour !== null && (
                        <div className="w-full h-1.5 bg-gray-700 rounded-full overflow-hidden">
                          <div
                            className="h-full rounded-full transition-all"
                            style={{ width: `${Math.min(pct, 100)}%`, backgroundColor: color }}
                          />
                        </div>
                      )}
                    </div>
                  );
                })}
                <div className="text-xs text-[var(--text-secondary)]">
                  Limits are rolling one-hour windows. Calls wait automatically when a limit is reached.
                </div>
              </div>
            </div>
          )}

          <div>
            <label className="text-xs text-[var(--text-secondary)] block mb-1">
              Change API Key
            </label>
            <div className="flex gap-2">
              <input
                type="password"
                value={newKey}
                onChange={(e) => setNewKey(e.target.value)}
                placeholder="New API key..."
                className="flex-1 px-3 py-2 rounded-lg bg-[var(--bg-secondary)] border border-gray-700
                           text-sm text-[var(--text-primary)] placeholder-gray-500
                           focus:outline-none focus:border-[var(--accent-blue)] transition"
              />
              <button
                onClick={handleChangeKey}
                disabled={saving || !newKey.trim()}
                className="px-4 py-2 rounded-lg bg-[var(--accent-blue)] text-white text-sm font-medium
                           hover:bg-blue-600 disabled:opacity-50 transition"
              >
                {saving ? "..." : "Save"}
              </button>
            </div>
          </div>

          <button
            onClick={handleClearKey}
            className="flex items-center gap-2 text-sm text-[var(--accent-red)] hover:text-red-400 transition"
          >
            <Trash2 className="w-3 h-3" />
            Clear API key & reset
          </button>
        </div>
      </div>
    </div>
  );
}
