import { useEffect } from "react";
import { useReplayStore } from "./hooks/useReplay";
import { ApiKeySetup } from "./components/input/ApiKeySetup";
import { ReplayInput } from "./components/input/ReplayInput";
import { StatCard } from "./components/stats/StatCard";
import { BoostPanel } from "./components/stats/BoostPanel";
import { PositioningPanel } from "./components/stats/PositioningPanel";
import { PlayerComparison } from "./components/stats/PlayerComparison";
import { AnalysisPanel } from "./components/stats/AnalysisPanel";
import { FieldRenderer } from "./components/field/FieldRenderer";
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
    <div className="min-h-screen bg-[var(--bg-primary)] p-4">
      {/* Header */}
      <div className="max-w-7xl mx-auto space-y-4">
        <div className="flex items-center justify-between">
          <h1 className="text-xl font-bold">RL Replay Review</h1>
          {replay && (
            <div className="text-sm text-[var(--text-secondary)]">
              {replay.title ?? replay.id} &middot;{" "}
              {replay.map_name ?? "Unknown Map"} &middot;{" "}
              {replay.duration ? `${Math.floor(replay.duration / 60)}:${String(replay.duration % 60).padStart(2, "0")}` : ""}
              {replay.overtime && " (OT)"}
            </div>
          )}
        </div>

        <ReplayInput />

        {replay && (
          <div className="grid grid-cols-12 gap-4">
            {/* Left Column: Teams & Stats */}
            <div className="col-span-4 space-y-4">
              {/* Team tabs / player selector */}
              <div className="flex flex-wrap gap-1">
                {allPlayers.map((p) => {
                  const team = teamColors.get(p.name ?? "") ?? "blue";
                  const isSelected = selectedPlayer?.name === p.name;
                  const base =
                    team === "blue"
                      ? isSelected
                        ? "bg-blue-600 text-white"
                        : "bg-blue-600/20 text-blue-400 hover:bg-blue-600/40"
                      : isSelected
                        ? "bg-orange-600 text-white"
                        : "bg-orange-600/20 text-orange-400 hover:bg-orange-600/40";
                  return (
                    <button
                      key={p.name}
                      onClick={() => selectPlayer(p)}
                      className={`px-3 py-1.5 rounded text-xs font-medium transition ${base}`}
                    >
                      {p.name ?? "?"}
                    </button>
                  );
                })}
              </div>

              {/* Blue Team */}
              <div className="space-y-2">
                <h3 className="text-sm font-semibold text-blue-400">
                  {replay.blue?.name ?? "Blue Team"}{" "}
                  <span className="text-[var(--text-secondary)]">
                    ({replay.blue?.stats?.core?.goals ?? 0})
                  </span>
                </h3>
                {bluePlayers.map((p) => (
                  <StatCard key={p.name} player={p} teamColor="blue" />
                ))}
              </div>

              {/* Orange Team */}
              <div className="space-y-2">
                <h3 className="text-sm font-semibold text-orange-400">
                  {replay.orange?.name ?? "Orange Team"}{" "}
                  <span className="text-[var(--text-secondary)]">
                    ({replay.orange?.stats?.core?.goals ?? 0})
                  </span>
                </h3>
                {orangePlayers.map((p) => (
                  <StatCard key={p.name} player={p} teamColor="orange" />
                ))}
              </div>
            </div>

            {/* Middle Column: Field + selected player details */}
            <div className="col-span-4 space-y-4">
              <FieldRenderer
                positioning={selectedPlayer?.stats?.positioning}
                playerName={selectedPlayer?.name ?? undefined}
                teamColor={selectedTeamColor}
                width={400}
                height={500}
              />

              {selectedPlayer && (
                <>
                  <BoostPanel player={selectedPlayer} />
                  <PositioningPanel player={selectedPlayer} />
                </>
              )}
            </div>

            {/* Right Column: Comparison + Analysis */}
            <div className="col-span-4 space-y-4">
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
        )}

        {!replay && (
          <div className="flex items-center justify-center h-64 text-[var(--text-secondary)]">
            Paste a ballchasing.com replay URL above to get started.
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
