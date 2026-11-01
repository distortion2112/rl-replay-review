import { useEffect, useRef } from "react";
import { Play, Pause, SkipBack, SkipForward } from "lucide-react";
import { usePlaybackStore } from "../../hooks/usePlayback";

export function TimelineScrubber() {
  const {
    frameData,
    currentFrameIndex,
    playing,
    speed,
    setFrameIndex,
    togglePlay,
    setSpeed,
    pause,
  } = usePlaybackStore();

  const animRef = useRef<number>(0);
  const lastTimeRef = useRef<number>(0);

  // Playback loop
  useEffect(() => {
    if (!playing || !frameData) return;

    const tick = (now: number) => {
      if (!lastTimeRef.current) lastTimeRef.current = now;
      const elapsed = (now - lastTimeRef.current) / 1000;
      lastTimeRef.current = now;

      const { currentFrameIndex, frameData, speed } = usePlaybackStore.getState();
      if (!frameData) return;

      const currentTime = frameData.frames[currentFrameIndex]?.time ?? 0;
      const targetTime = currentTime + elapsed * speed;

      // Find next frame at target time
      let nextIdx = currentFrameIndex;
      while (
        nextIdx < frameData.frames.length - 1 &&
        frameData.frames[nextIdx].time < targetTime
      ) {
        nextIdx++;
      }

      if (nextIdx >= frameData.frames.length - 1) {
        usePlaybackStore.getState().pause();
        return;
      }

      usePlaybackStore.getState().setFrameIndex(nextIdx);
      animRef.current = requestAnimationFrame(tick);
    };

    lastTimeRef.current = 0;
    animRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(animRef.current);
  }, [playing, frameData]);

  if (!frameData) return null;

  const totalFrames = frameData.frames.length;
  const currentTime = frameData.frames[currentFrameIndex]?.time ?? 0;
  const duration = frameData.duration_seconds;

  const formatTime = (t: number) => {
    const m = Math.floor(t / 60);
    const s = Math.floor(t % 60);
    return `${m}:${String(s).padStart(2, "0")}`;
  };

  return (
    <div className="bg-[var(--bg-card)] rounded-lg p-3 space-y-2">
      {/* Timeline bar */}
      <div className="relative">
        <input
          type="range"
          min={0}
          max={totalFrames - 1}
          value={currentFrameIndex}
          onChange={(e) => {
            setFrameIndex(Number(e.target.value));
            if (playing) pause();
          }}
          className="w-full h-2 appearance-none bg-[var(--bg-secondary)] rounded-full cursor-pointer
                     [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3
                     [&::-webkit-slider-thumb]:h-3 [&::-webkit-slider-thumb]:rounded-full
                     [&::-webkit-slider-thumb]:bg-[var(--accent-blue)]"
        />
        {/* Goal markers */}
        {frameData.goals.map((g, i) => {
          const goalPct = duration > 0 ? (g.time / duration) * 100 : 0;
          return (
            <div
              key={i}
              className="absolute top-0 w-1 h-2 bg-[var(--accent-yellow)] rounded"
              style={{ left: `${goalPct}%` }}
              title={`Goal at ${formatTime(g.time)}`}
            />
          );
        })}
      </div>

      {/* Controls */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <button
            onClick={() => setFrameIndex(0)}
            className="p-1.5 rounded hover:bg-[var(--bg-secondary)] transition"
          >
            <SkipBack className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={togglePlay}
            className="p-2 rounded-full bg-[var(--accent-blue)] hover:bg-blue-600 transition"
          >
            {playing ? (
              <Pause className="w-4 h-4" />
            ) : (
              <Play className="w-4 h-4 ml-0.5" />
            )}
          </button>
          <button
            onClick={() => setFrameIndex(totalFrames - 1)}
            className="p-1.5 rounded hover:bg-[var(--bg-secondary)] transition"
          >
            <SkipForward className="w-3.5 h-3.5" />
          </button>
        </div>

        <div className="text-xs text-[var(--text-secondary)] font-mono">
          {formatTime(currentTime)} / {formatTime(duration)}
        </div>

        <div className="flex items-center gap-1">
          {[0.25, 0.5, 1, 2].map((s) => (
            <button
              key={s}
              onClick={() => setSpeed(s)}
              className={`px-2 py-0.5 rounded text-xs font-medium transition ${
                speed === s
                  ? "bg-[var(--accent-blue)] text-white"
                  : "text-[var(--text-secondary)] hover:bg-[var(--bg-secondary)]"
              }`}
            >
              {s}x
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
