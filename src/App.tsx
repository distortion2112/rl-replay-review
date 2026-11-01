import { useEffect } from "react";
import { useReplayStore } from "./hooks/useReplay";
import { ApiKeySetup } from "./components/input/ApiKeySetup";
import { ReplayInput } from "./components/input/ReplayInput";
import { SettingsMenu } from "./components/input/SettingsMenu";
import { StatCard } from "./components/stats/StatCard";
import { BoostPanel } from "./components/stats/BoostPanel";
import { PositioningPanel } from "./components/stats/PositioningPanel";
import { PlayerComparison } from "./components/stats/PlayerComparison";
import { AnalysisPanel } from "./components/stats/AnalysisPanel";
import { FieldRenderer } from "./components/field/FieldRenderer";
import { BoostChart } from "./components/charts/BoostChart";
import { MovementChart } from "./components/charts/MovementChart";
import { Trophy } from "lucide-react";
import "./styles.css";

function App() {
  const {
    apiKey,
    replay,
    selectedPlayer,
    rotationInsights,
    boostInsights,
    initApiKey,
    selectPlayer,
  } = useReplayStore();

  useEffect(() => {
    initApiKey();
  }, []);

  if (apiKey === null && !replay) {
    return <ApiKeySetup />;
  }

  const bluePlayers = replay?.blue?.players ?? [];
  const orangePlayers = replay?.orange?.players ?? [];
  const allPlayers = [...bluePlayers, ...orangePlayers];
  const teamColors = new Map<string, "blue" | "orange">();
  bluePlayers.forEach((p) => p.name && teamColors.set(p.name, "blue"));
  orangePlayers.forEach((p) => p.name && teamColors.set(p.name, "orange"));

  const selectedTeamColor = teamColors.get(selectedPlayer?.name ?? "") ?? "blue";

  return (
    <div className="min-h-screen bg-[var(--bg-primary)]">
      {/* Header */}
      <header className="sticky top-0 z-10 bg-[var(--bg-primary)]/90 backdrop-blur border-b border-gray-800/50">
        <div className="max-w-[1600px] mx-auto px-4 py-3 flex items-center gap-4">
          <div className="flex items-center gap-2 shrink-0">
            <Trophy className="w-5 h-5 text-[var(--accent-blue)]" />
            <h1 className="text-lg font-bold">RL Replay Review</h1>
          </div>

          <div className="flex-1">
            <ReplayInput />
          </div>

          {replay && (
            <div className="text-xs text-[var(--text-secondary)] shrink-0 text-right">
              <div>{replay.title ?? replay.id}</div>
              <div>
                {replay.map_name ?? "Unknown Map"} &middot;{" "}
                {replay.duration
                  ? `${Math.floor(replay.duration / 60)}:${String(replay.duration % 60).padStart(2, "0")}`
                  : ""}
                {replay.overtime && " (OT)"}
              </div>
            </div>
          )}

          <SettingsMenu />
        </div>
      </header>

      <main className="max-w-[1600px] mx-auto px-4 py-4">
        {replay ? (
          <>
            {/* Score banner */}
            <div className="flex items-center justify-center gap-8 mb-4 py-3 bg-[var(--bg-secondary)] rounded-lg">
              <div className="text-right">
                <div className="text-sm font-medium text-blue-400">
                  {replay.blue?.name ?? "Blue Team"}
                </div>
                <div className="text-3xl font-bold">
                  {replay.blue?.stats?.core?.goals ?? 0}
                </div>
              </div>
              <div className="text-[var(--text-secondary)] text-sm">vs</div>
              <div className="text-left">
                <div className="text-sm font-medium text-orange-400">
                  {replay.orange?.name ?? "Orange Team"}
                </div>
                <div className="text-3xl font-bold">
                  {replay.orange?.stats?.core?.goals ?? 0}
                </div>
              </div>
            </div>

            {/* Player selector */}
            <div className="flex flex-wrap gap-1.5 mb-4">
              {allPlayers.map((p) => {
                const team = teamColors.get(p.name ?? "") ?? "blue";
                const isSelected = selectedPlayer?.name === p.name;
                const base =
                  team === "blue"
                    ? isSelected
                      ? "bg-blue-600 text-white shadow-lg shadow-blue-600/20"
                      : "bg-blue-600/20 text-blue-400 hover:bg-blue-600/40"
                    : isSelected
                      ? "bg-orange-600 text-white shadow-lg shadow-orange-600/20"
                      : "bg-orange-600/20 text-orange-400 hover:bg-orange-600/40";
                return (
                  <button
                    key={p.name}
                    onClick={() => selectPlayer(p)}
                    className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${base}`}
                  >
                    {p.name ?? "?"}
                    {p.mvp && " (MVP)"}
                  </button>
                );
              })}
            </div>

            <div className="grid grid-cols-12 gap-4">
              {/* Left: Team stat cards */}
              <div className="col-span-3 space-y-4 overflow-y-auto max-h-[calc(100vh-220px)]">
                <div className="space-y-2">
                  <h3 className="text-xs font-semibold text-blue-400 uppercase tracking-wide">
                    {replay.blue?.name ?? "Blue Team"}
                  </h3>
                  {bluePlayers.map((p) => (
                    <div
                      key={p.name}
                      onClick={() => selectPlayer(p)}
                      className="cursor-pointer"
                    >
                      <StatCard player={p} teamColor="blue" />
                    </div>
                  ))}
                </div>
                <div className="space-y-2">
                  <h3 className="text-xs font-semibold text-orange-400 uppercase tracking-wide">
                    {replay.orange?.name ?? "Orange Team"}
                  </h3>
                  {orangePlayers.map((p) => (
                    <div
                      key={p.name}
                      onClick={() => selectPlayer(p)}
                      className="cursor-pointer"
                    >
                      <StatCard player={p} teamColor="orange" />
                    </div>
                  ))}
                </div>
              </div>

              {/* Middle: Field + selected player detail */}
              <div className="col-span-5 space-y-4">
                <FieldRenderer
                  positioning={selectedPlayer?.stats?.positioning}
                  playerName={selectedPlayer?.name ?? undefined}
                  teamColor={selectedTeamColor}
                />

                {selectedPlayer && (
                  <>
                    <BoostPanel player={selectedPlayer} />
                    <BoostChart player={selectedPlayer} />
                    <MovementChart player={selectedPlayer} />
                    <PositioningPanel player={selectedPlayer} />
                  </>
                )}
              </div>

              {/* Right: Comparison + Analysis */}
              <div className="col-span-4 space-y-4 overflow-y-auto max-h-[calc(100vh-220px)]">
                <PlayerComparison
                  players={allPlayers}
                  teamColors={teamColors}
                />
                <AnalysisPanel
                  rotationInsights={rotationInsights}
                  boostInsights={boostInsights}
                />
              </div>
            </div>
          </>
        ) : (
          <div className="flex flex-col items-center justify-center h-[60vh] text-center">
            <Trophy className="w-16 h-16 text-gray-700 mb-4" />
            <h2 className="text-xl font-semibold text-[var(--text-secondary)] mb-2">
              No Replay Loaded
            </h2>
            <p className="text-gray-600 max-w-md">
              Paste a ballchasing.com replay URL or ID in the search bar to
              start analyzing.
            </p>
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
