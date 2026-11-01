import { useState } from "react";
import { Search, Loader2 } from "lucide-react";
import { useReplayStore } from "../../hooks/useReplay";

export function ReplayInput() {
  const [input, setInput] = useState("");
  const { loadReplay, loading, error } = useReplayStore();

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim() || loading) return;
    loadReplay(input);
  };

  return (
    <div className="space-y-2">
      <form onSubmit={handleSubmit} className="flex gap-2">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-500" />
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="Paste ballchasing.com URL or replay ID..."
            className="w-full pl-10 pr-4 py-2.5 rounded-lg bg-[var(--bg-secondary)] border border-gray-700
                       text-[var(--text-primary)] placeholder-gray-500 focus:outline-none
                       focus:border-[var(--accent-blue)] transition text-sm"
          />
        </div>
        <button
          type="submit"
          disabled={loading || !input.trim()}
          className="px-5 py-2.5 rounded-lg bg-[var(--accent-blue)] text-white font-medium text-sm
                     hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition
                     flex items-center gap-2"
        >
          {loading ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            "Analyze"
          )}
        </button>
      </form>
      {error && (
        <p className="text-sm text-[var(--accent-red)] px-1">{error}</p>
      )}
    </div>
  );
}
