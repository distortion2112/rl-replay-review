import { useEffect, useState } from "react";
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
import { LiveFieldView } from "./components/field/LiveFieldView";
import { HeatmapControls, useHeatmapState, useHeatmapGrid } from "./components/field/HeatmapOverlay";
import { TrailControls } from "./components/field/TrailControls";
import { BoostChart } from "./components/charts/BoostChart";
import { MovementChart } from "./components/charts/MovementChart";
import { BoostTimelineChart } from "./components/charts/BoostTimelineChart";
import { TimelineScrubber } from "./components/timeline/TimelineScrubber";
import { PlaybackControls } from "./components/timeline/PlaybackControls";
import { GoalReplays } from "./components/timeline/GoalReplays";
import { usePlaybackStore } from "./hooks/usePlayback";
import { GraduationCap, RotateCcw, Trophy } from "lucide-react";
import { CoachingView } from "./components/coaching/CoachingView";
import { useKeyboard } from "./hooks/useKeyboard";
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
  const hasFrameData = usePlaybackStore((s) => !!s.frameData);
  const resetFrameData = usePlaybackStore((s) => s.setFrameData);
  const heatmapState = useHeatmapState();
  const heatmapGrid = useHeatmapGrid(heatmapState.mode, heatmapState.selectedIdx);
  const [showTrails, setShowTrails] = useState(true);
  const [trailSeconds, setTrailSeconds] = useState(5);
  const [view, setView] = useState<"replay" | "coaching">("replay");

  useKeyboard();

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
    <div className="h-screen flex flex-col bg-[var(--bg-primary)] overflow-hidden">
      {/* Header - always visible */}
      <header className="shrink-0 bg-[var(--bg-secondary)] border-b border-gray-800/50 z-10">
        <div className="max-w-[1600px] mx-auto px-4 py-2 flex items-center gap-4">
          <div className="flex items-center gap-2 shrink-0">
            <Trophy className="w-5 h-5 text-[var(--accent-blue)]" />
            <h1 className="text-lg font-bold">RL Replay Review</h1>
          </div>

          <nav className="flex gap-1 shrink-0" aria-label="Views">
            {([
              ["replay", "Replay Review", Trophy],
              ["coaching", "Coaching", GraduationCap],
            ] as const).map(([key, label, Icon]) => (
              <button
                key={key}
                onClick={() => setView(key)}
                aria-current={view === key ? "page" : undefined}
                className={`px-3 py-1 rounded text-xs font-medium flex items-center gap-1 transition ${
                  view === key
                    ? "bg-[var(--accent-blue)] text-white"
                    : "text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-card)]"
                }`}
              >
                <Icon className="w-3 h-3" />
                {label}
              </button>
            ))}
          </nav>

          <div className="flex-1 max-w-xl">{view === "replay" && <ReplayInput />}</div>

          {view === "replay" && replay && (
            <div className="text-xs text-[var(--text-secondary)] shrink-0 text-right">
              <div className="font-medium text-[var(--text-primary)]">
                {replay.title ?? replay.id}
              </div>
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

      {view === "coaching" ? (
        <div className="flex-1 min-h-0">
          <CoachingView />
        </div>
      ) : replay ? (
        <>
          {/* Score bar + player selector - always visible */}
          <div className="shrink-0 bg-[var(--bg-primary)] border-b border-gray-800/30 px-4 py-2">
            <div className="max-w-[1600px] mx-auto">
              {/* Score */}
              <div className="flex items-center justify-center gap-6 mb-2">
                <div className="text-right">
                  <span className="text-xs font-medium text-blue-400">
                    {replay.blue?.name ?? "Blue"}
                  </span>
                  <span className="text-2xl font-bold ml-2">
                    {replay.blue?.stats?.core?.goals ?? 0}
                  </span>
                </div>
                <span className="text-[var(--text-secondary)] text-xs">vs</span>
                <div className="text-left">
                  <span className="text-2xl font-bold mr-2">
                    {replay.orange?.stats?.core?.goals ?? 0}
                  </span>
                  <span className="text-xs font-medium text-orange-400">
                    {replay.orange?.name ?? "Orange"}
                  </span>
                </div>
              </div>

              {/* Player selector + frame data toggle */}
              <div className="flex items-center gap-2">
                <div className="flex flex-wrap gap-1 flex-1">
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
                        className={`px-3 py-1 rounded text-xs font-medium transition-all ${base}`}
                      >
                        {p.name ?? "?"}
                        {p.mvp && " *"}
                      </button>
                    );
                  })}
                </div>
                {hasFrameData && (
                  <button
                    onClick={() => resetFrameData(null)}
                    className="px-2 py-1 rounded text-xs text-[var(--text-secondary)] hover:text-[var(--text-primary)]
                               hover:bg-[var(--bg-card)] transition flex items-center gap-1"
                    title="Back to stats view"
                  >
                    <RotateCcw className="w-3 h-3" />
                    Stats View
                  </button>
                )}
              </div>
            </div>
          </div>

          {/* Main content - fills remaining height */}
          <div className="flex-1 min-h-0 px-4 py-2">
            <div className="max-w-[1600px] mx-auto h-full grid grid-cols-12 gap-3">
              {/* Left: Team stat cards */}
              <div className="col-span-3 overflow-y-auto pr-1">
                <div className="space-y-2">
                  <h3 className="text-xs font-semibold text-blue-400 uppercase tracking-wide sticky top-0 bg-[var(--bg-primary)] py-1">
                    {replay.blue?.name ?? "Blue Team"}
                  </h3>
                  {bluePlayers.map((p) => (
                    <div
                      key={p.name}
                      onClick={() => selectPlayer(p)}
                      className={`cursor-pointer rounded-lg transition-all ${
                        selectedPlayer?.name === p.name ? "ring-1 ring-blue-500/50" : ""
                      }`}
                    >
                      <StatCard player={p} teamColor="blue" />
                    </div>
                  ))}
                </div>
                <div className="space-y-2 mt-4">
                  <h3 className="text-xs font-semibold text-orange-400 uppercase tracking-wide sticky top-0 bg-[var(--bg-primary)] py-1">
                    {replay.orange?.name ?? "Orange Team"}
                  </h3>
                  {orangePlayers.map((p) => (
                    <div
                      key={p.name}
                      onClick={() => selectPlayer(p)}
                      className={`cursor-pointer rounded-lg transition-all ${
                        selectedPlayer?.name === p.name ? "ring-1 ring-orange-500/50" : ""
                      }`}
                    >
                      <StatCard player={p} teamColor="orange" />
                    </div>
                  ))}
                </div>
              </div>

              {/* Middle: Field + controls */}
              <div className="col-span-5 overflow-y-auto pr-1" data-scroll-middle>
                {hasFrameData ? (
                  <div className="space-y-3">
                    <LiveFieldView
                      heatmapGrid={heatmapGrid}
                      showTrails={showTrails}
                      trailSeconds={trailSeconds}
                    />
                    <TimelineScrubber />
                    <GoalReplays />
                    <div className="grid grid-cols-2 gap-2">
                      <TrailControls
                        showTrails={showTrails}
                        setShowTrails={setShowTrails}
                        trailSeconds={trailSeconds}
                        setTrailSeconds={setTrailSeconds}
                      />
                      <HeatmapControls {...heatmapState} />
                    </div>
                    <BoostTimelineChart />
                  </div>
                ) : (
                  <div className="space-y-3">
                    <FieldRenderer
                      positioning={selectedPlayer?.stats?.positioning}
                      playerName={selectedPlayer?.name ?? undefined}
                      teamColor={selectedTeamColor}
                    />
                    <PlaybackControls />
                  </div>
                )}

                {selectedPlayer && (
                  <div className="space-y-3 mt-3">
                    <BoostPanel player={selectedPlayer} />
                    <BoostChart player={selectedPlayer} />
                    <MovementChart player={selectedPlayer} />
                    <PositioningPanel player={selectedPlayer} />
                  </div>
                )}
              </div>

              {/* Right: Comparison + Analysis */}
              <div className="col-span-4 overflow-y-auto pl-1">
                <div className="space-y-3">
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
            </div>
          </div>
        </>
      ) : (
        <div className="flex-1 flex flex-col items-center justify-center text-center">
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
    </div>
  );
}

export default App;
