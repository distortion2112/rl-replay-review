import { useState } from "react";
import { Search, UserRound } from "lucide-react";
import { useReplayStore } from "../../hooks/useReplay";
import { useCoachingStore, type CoachingProfile } from "../../hooks/useCoaching";
import { RANKS, isSiteSearchUrl, nameSearchText, nameSearchUrl, parsePlayerInput, rankName } from "../../lib/coaching";
import { searchSiteReplays } from "../../lib/tauri-commands";
import type { SearchIdentity } from "../../types/replay";

/** What a name search found: who, and what was left out. */
interface Found {
  identities: SearchIdentity[];
  replays: number;
  ignored: string[];
}

/** Who to coach: player id and/or a name search, rank and how many recent games. */
export function ProfileForm({ onDone }: { onDone?: () => void }) {
  const { profile, setProfile } = useCoachingStore();
  const replay = useReplayStore((s) => s.replay);
  // A profile saved from a name search alone has no typed id: its primary id came from the ticks
  const typedSaved = profile && !(profile.searchUrl && profile.idFromSearch);
  const [playerText, setPlayerText] = useState(typedSaved ? (profile?.playerId ?? "") : "");
  const [nameText, setNameText] = useState(profile?.searchUrl ? nameSearchText(profile.searchUrl) : "");
  const [name, setName] = useState<string | null>(profile?.name ?? null);
  const [rank, setRank] = useState(profile?.rank ?? "");
  const [games, setGames] = useState(profile?.games ?? 30);
  const savedIds = profile?.searchUrl
    ? [...(typedSaved ? [] : [profile.playerId]), ...(profile.aliases ?? [])]
    : [];
  const [found, setFound] = useState<Found | null>(null);
  const [chosen, setChosen] = useState<string[]>(savedIds);
  const [finding, setFinding] = useState(false);
  const [findError, setFindError] = useState<string | null>(null);

  const typedId = parsePlayerInput(playerText);
  const searchUrl = nameSearchUrl(nameText);
  // Ticked accounts, most games first
  const ticked = found ? found.identities.map((i) => i.player_id).filter((id) => chosen.includes(id)) : chosen;
  // A typed id is the primary one; otherwise the ticked account with the most games
  const playerId = typedId ?? ticked[0] ?? null;
  const aliases = searchUrl ? ticked.filter((id) => id !== playerId) : [];
  const replayPlayers = [...(replay?.blue?.players ?? []), ...(replay?.orange?.players ?? [])].filter(
    (p) => p.id?.platform && p.id?.id,
  );
  const playerOk = playerText.trim() === "" || typedId !== null;
  const nameOk = nameText.trim() === "" || searchUrl !== null;
  const valid = playerId !== null && playerOk && nameOk && rank !== "" && games >= 10 && games <= 200;

  const resetSearch = () => {
    setFound(null);
    setChosen([]);
    setFindError(null);
  };

  const find = async () => {
    if (!searchUrl) return;
    setFinding(true);
    setFindError(null);
    try {
      const r = await searchSiteReplays(searchUrl, Math.max(games, 10));
      setFound({ identities: r.identities, replays: r.replays.length, ignored: r.ignored });
      // Pre-tick saved ids and the typed id; with neither, the account seen in the most games
      const keep = r.identities
        .map((i) => i.player_id)
        .filter((id) => chosen.includes(id) || id === typedId);
      setChosen(keep.length > 0 || typedId ? keep : r.identities.slice(0, 1).map((i) => i.player_id));
      if (!name && r.identities[0]) setName(r.identities[0].name);
    } catch (e) {
      setFound(null);
      setFindError(e instanceof Error ? e.message : String(e));
    } finally {
      setFinding(false);
    }
  };

  const save = () => {
    if (!valid || !playerId) return;
    const next: CoachingProfile = searchUrl
      ? { playerId, name, rank, games, searchUrl, aliases, idFromSearch: typedId === null }
      : { playerId, name, rank, games };
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
        <span className="text-xs text-[var(--text-secondary)]">Player id</span>
        <input
          value={playerText}
          onChange={(e) => {
            const v = e.target.value;
            // A search link pasted here belongs in the name box
            if (isSiteSearchUrl(v)) {
              setNameText(v.trim());
              setPlayerText("");
              resetSearch();
              return;
            }
            setPlayerText(v);
            if (!nameText) setName(null);
          }}
          placeholder="ballchasing player link, or epic:… / steam:…"
          className="w-full bg-[var(--bg-secondary)] rounded px-3 py-2 text-sm font-mono outline-none focus:ring-1 focus:ring-[var(--accent-blue)]"
        />
        {!playerOk && (
          <span className="text-xs text-[var(--accent-orange)]">
            Paste a link like ballchasing.com/player/epic/… or an id like steam:76561198…
          </span>
        )}
      </label>

      <div className="space-y-2">
        <span className="text-xs text-[var(--text-secondary)]">
          Player name {typedId ? "(optional: adds games from other accounts)" : ""}
        </span>
        <div className="flex gap-2">
          <input
            value={nameText}
            onChange={(e) => {
              setNameText(e.target.value);
              resetSearch();
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && searchUrl && !finding) void find();
            }}
            placeholder="your in-game name, or a ballchasing search link"
            className="flex-1 min-w-0 bg-[var(--bg-secondary)] rounded px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-[var(--accent-blue)]"
          />
          <button
            onClick={find}
            disabled={!searchUrl || finding}
            className="shrink-0 flex items-center gap-1 px-3 py-2 rounded bg-[var(--accent-blue)] text-white text-sm font-medium disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <Search className="w-4 h-4" />
            {finding ? "Searching…" : found ? "Search again" : "Find games"}
          </button>
        </div>
        {!nameOk && (
          <span className="text-xs text-[var(--accent-orange)]">
            Type a name, or paste a link from ballchasing's replay search (ballchasing.com/?player-name=…)
          </span>
        )}
        {(findError || found || chosen.length > 0) && (
          <div className="space-y-2 bg-[var(--bg-secondary)] rounded p-3">
            {findError && <p className="text-xs text-[var(--accent-orange)]">{findError}</p>}
            {found && found.identities.length === 0 && (
              <p className="text-xs text-[var(--accent-orange)]">
                {found.replays} ranked 2v2 games found, but no player in them has exactly that name.
              </p>
            )}
            {found && found.identities.length > 0 && (
              <div className="space-y-1">
                <span className="text-xs text-[var(--text-secondary)]">
                  Accounts with this name in the {found.replays} newest ranked 2v2 games. A name can belong
                  to more than one account, or to someone else: tick the ones that are you.
                </span>
                {found.identities.map((i) => (
                  <label key={i.player_id} className="flex items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={chosen.includes(i.player_id) || i.player_id === typedId}
                      disabled={i.player_id === typedId}
                      onChange={(e) =>
                        setChosen((c) => (e.target.checked ? [...c, i.player_id] : c.filter((x) => x !== i.player_id)))
                      }
                    />
                    <span>{i.name}</span>
                    <span className="font-mono text-xs text-[var(--text-secondary)]">{i.player_id}</span>
                    <span className="text-xs text-[var(--text-secondary)]">
                      · {i.games} game{i.games === 1 ? "" : "s"}
                      {i.player_id === typedId ? " · your player id" : ""}
                    </span>
                  </label>
                ))}
              </div>
            )}
            {!found && !findError && chosen.length > 0 && (
              <p className="text-xs text-[var(--text-secondary)]">Saved accounts: {chosen.join(", ")}</p>
            )}
            {found && found.ignored.length > 0 && (
              <p className="text-xs text-[var(--text-secondary)]">Not applied: {found.ignored.join("; ")}</p>
            )}
          </div>
        )}
        {!typedId && !found && searchUrl && (
          <p className="text-xs text-[var(--text-secondary)]">
            No player id? Find games and tick your accounts instead.
          </p>
        )}
      </div>

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
