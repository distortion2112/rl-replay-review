import { useState } from "react";
import { Settings, X, KeyRound, Trash2 } from "lucide-react";
import { useReplayStore } from "../../hooks/useReplay";

export function SettingsMenu() {
  const [open, setOpen] = useState(false);
  const [newKey, setNewKey] = useState("");
  const [saving, setSaving] = useState(false);
  const { apiKey, saveApiKey } = useReplayStore();

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
