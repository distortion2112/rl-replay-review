import { Target } from "lucide-react";
import { usePlaybackStore } from "../../hooks/usePlayback";

export function GoalReplays() {
  const { frameData, setFrameIndex, play, pause, playing } = usePlaybackStore();

  if (!frameData || frameData.goals.length === 0) return null;

  const formatTime = (t: number) => {
    const m = Math.floor(t / 60);
    const s = Math.floor(t % 60);
    return `${m}:${String(s).padStart(2, "0")}`;
  };

  const jumpToGoal = (goalTime: number) => {
    // Jump to 8 seconds before the goal
    const targetTime = Math.max(0, goalTime - 8);

    // Find the frame closest to target time
    let bestIdx = 0;
    let bestDiff = Infinity;
    for (let i = 0; i < frameData.frames.length; i++) {
      const diff = Math.abs(frameData.frames[i].time - targetTime);
      if (diff < bestDiff) {
        bestDiff = diff;
        bestIdx = i;
      }
    }

    if (playing) pause();
    setFrameIndex(bestIdx);
    // Auto-play after a brief moment
    setTimeout(() => play(), 100);
  };

  return (
    <div className="bg-[var(--bg-card)] rounded-lg p-3 space-y-2">
      <div className="flex items-center gap-2">
        <Target className="w-3.5 h-3.5 text-[var(--accent-yellow)]" />
        <span className="text-xs font-semibold text-[var(--text-secondary)] uppercase tracking-wide">
          Goals
        </span>
      </div>
      <div className="flex flex-wrap gap-1.5">
        {frameData.goals.map((goal, i) => (
          <button
            key={i}
            onClick={() => jumpToGoal(goal.time)}
            className="px-3 py-1.5 rounded-lg bg-[var(--bg-secondary)] hover:bg-[var(--accent-yellow)]/20
                       text-xs font-medium text-[var(--text-primary)] transition
                       flex items-center gap-1.5 border border-transparent hover:border-[var(--accent-yellow)]/30"
          >
            <span className="text-[var(--accent-yellow)] font-bold">#{i + 1}</span>
            <span className="text-[var(--text-secondary)]">{formatTime(goal.time)}</span>
          </button>
        ))}
      </div>
      <p className="text-[10px] text-[var(--text-secondary)]/50">
        Click a goal to replay the 8 seconds leading up to it
      </p>
    </div>
  );
}
