import { useState } from "react";
import { UserRound } from "lucide-react";
import { useReplayStore } from "../../hooks/useReplay";
import { useCoachingStore, type CoachingProfile } from "../../hooks/useCoaching";
import { RANKS, parsePlayerInput, rankName } from "../../lib/coaching";

/** Who to coach: player id, rank and how many recent games. */
export function ProfileForm({ onDone }: { onDone?: () => void }) {
  const { profile, setProfile } = useCoachingStore();
  const replay = useReplayStore((s) => s.replay);
  const [playerText, setPlayerText] = useState(profile?.playerId ?? "");
  const [name, setName] = useState<string | null>(profile?.name ?? null);
  const [rank, setRank] = useState(profile?.rank ?? "");
  const [games, setGames] = useState(profile?.games ?? 30);

  const playerId = parsePlayerInput(playerText);
  const replayPlayers = [...(replay?.blue?.players ?? []), ...(replay?.orange?.players ?? [])].filter(
    (p) => p.id?.platform && p.id?.id,
  );
  const valid = playerId !== null && rank !== "" && games >= 10 && games <= 200;

  const save = () => {
    if (!valid || !playerId) return;
    const next: CoachingProfile = { playerId, name, rank, games };
    setProfile(next);
    onDone?.();
  };

  return (
    <div className="bg-[var(--bg-card)] rounded-lg p-4 space-y-4 max-w-xl">
      <div className="flex items-center gap-2">
        <UserRound className="w-4 h-4 text-[var(--accent-blue)]" />
        <h3 className="font-semibold">Who should we coach?</h3>
      </div>

      <label className="block space-y-1">
        <span className="text-xs text-[var(--text-secondary)]">Player</span>
        <input
          value={playerText}
          onChange={(e) => {
            setPlayerText(e.target.value);
            setName(null);
          }}
          placeholder="ballchasing player link, or epic:… / steam:…"
          className="w-full bg-[var(--bg-secondary)] rounded px-3 py-2 text-sm font-mono outline-none focus:ring-1 focus:ring-[var(--accent-blue)]"
        />
        {playerText && !playerId && (
          <span className="text-xs text-[var(--accent-orange)]">
            Paste a link like ballchasing.com/player/epic/… or an id like steam:76561198…
          </span>
        )}
      </label>

      {replayPlayers.length > 0 && (
        <div className="space-y-1">
          <span className="text-xs text-[var(--text-secondary)]">Or pick from the loaded replay</span>
          <div className="flex flex-wrap gap-1">
            {replayPlayers.map((p) => {
              const id = `${p.id?.platform}:${p.id?.id}`;
              return (
                <button
                  key={id}
                  onClick={() => {
                    setPlayerText(id);
                    setName(p.name ?? null);
                  }}
                  className={`px-2 py-1 rounded text-xs transition ${
                    playerId === id
                      ? "bg-[var(--accent-blue)] text-white"
                      : "bg-[var(--bg-secondary)] hover:bg-gray-700"
                  }`}
                >
                  {p.name ?? id}
                </button>
              );
            })}
          </div>
        </div>
      )}

      <div className="grid grid-cols-2 gap-3">
        <label className="block space-y-1">
          <span className="text-xs text-[var(--text-secondary)]">Your 2v2 rank</span>
          <select
            value={rank}
            onChange={(e) => setRank(e.target.value)}
            className="w-full bg-[var(--bg-secondary)] rounded px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-[var(--accent-blue)]"
          >
            <option value="">Choose…</option>
            {RANKS.map((r) => (
              <option key={r} value={r}>
                {rankName(r)}
              </option>
            ))}
          </select>
        </label>
        <label className="block space-y-1">
          <span className="text-xs text-[var(--text-secondary)]">Recent games to analyze</span>
          <input
            type="number"
            min={10}
            max={200}
            value={games}
            onChange={(e) => setGames(Number(e.target.value))}
            className="w-full bg-[var(--bg-secondary)] rounded px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-[var(--accent-blue)]"
          />
        </label>
      </div>
      <p className="text-xs text-[var(--text-secondary)]">
        ballchasing no longer reports ranks, so pick yours from the in-game ranked screen. Coaching needs at
        least 10 games; more games give tighter ranges.
      </p>

      <button
        onClick={save}
        disabled={!valid}
        className="px-4 py-2 rounded bg-[var(--accent-blue)] text-white text-sm font-medium disabled:opacity-40 disabled:cursor-not-allowed"
      >
        Save
      </button>
    </div>
  );
}
