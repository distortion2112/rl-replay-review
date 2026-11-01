import { useState } from "react";
import { KeyRound } from "lucide-react";
import { useReplayStore } from "../../hooks/useReplay";

export function ApiKeySetup() {
  const [key, setKey] = useState("");
  const [saving, setSaving] = useState(false);
  const saveApiKey = useReplayStore((s) => s.saveApiKey);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!key.trim()) return;
    setSaving(true);
    await saveApiKey(key.trim());
    setSaving(false);
  };

  return (
    <div className="flex items-center justify-center min-h-screen bg-[var(--bg-primary)]">
      <div className="bg-[var(--bg-card)] rounded-xl p-8 max-w-md w-full shadow-2xl">
        <div className="flex items-center gap-3 mb-6">
          <KeyRound className="w-8 h-8 text-[var(--accent-blue)]" />
          <h1 className="text-2xl font-bold">RL Replay Review</h1>
        </div>
        <p className="text-[var(--text-secondary)] mb-6">
          Enter your ballchasing.com API key to get started. You can get a free
          key from{" "}
          <span className="text-[var(--accent-blue)]">
            ballchasing.com/upload
          </span>
          .
        </p>
        <form onSubmit={handleSubmit} className="space-y-4">
          <input
            type="password"
            value={key}
            onChange={(e) => setKey(e.target.value)}
            placeholder="Your API key..."
            className="w-full px-4 py-3 rounded-lg bg-[var(--bg-secondary)] border border-gray-700
                       text-[var(--text-primary)] placeholder-gray-500 focus:outline-none
                       focus:border-[var(--accent-blue)] transition"
          />
          <button
            type="submit"
            disabled={saving || !key.trim()}
            className="w-full py-3 rounded-lg bg-[var(--accent-blue)] text-white font-medium
                       hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition"
          >
            {saving ? "Saving..." : "Save & Continue"}
          </button>
        </form>
      </div>
    </div>
  );
}
