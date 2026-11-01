import { Loader2, Film, FlaskConical } from "lucide-react";
import { usePlaybackStore } from "../../hooks/usePlayback";
import { useReplayStore } from "../../hooks/useReplay";
import { downloadAndParseReplay } from "../../lib/tauri-commands";
import { generateTestFrameData } from "../../lib/test-data";

export function PlaybackControls() {
  const { frameData, loading, error, setFrameData, setLoading, setError } =
    usePlaybackStore();
  const replay = useReplayStore((s) => s.replay);

  const handleLoad = async () => {
    if (!replay) return;
    setLoading(true);
    setError(null);
    try {
      const data = await downloadAndParseReplay(replay.id);
      console.log("[FrameData]", {
        players: data.players,
        totalFrames: data.total_frames,
        duration: data.duration_seconds,
        sampleFrame: data.frames[Math.floor(data.frames.length / 2)],
      });
      setFrameData(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleTestMode = () => {
    const data = generateTestFrameData();
    console.log("[TestFrameData]", {
      players: data.players,
      totalFrames: data.total_frames,
      sampleFrame: data.frames[0],
    });
    setFrameData(data);
  };

  if (frameData) return null;

  return (
    <div className="bg-[var(--bg-card)] rounded-lg p-4 text-center space-y-2">
      <Film className="w-8 h-8 text-[var(--text-secondary)] mx-auto" />
      <p className="text-sm text-[var(--text-secondary)]">
        Load frame-by-frame data to see player positions, timeline playback,
        and detailed boost tracking.
      </p>
      <div className="flex gap-2 justify-center">
        <button
          onClick={handleLoad}
          disabled={loading || !replay}
          className="px-4 py-2 rounded-lg bg-[var(--accent-blue)] text-white text-sm font-medium
                     hover:bg-blue-600 disabled:opacity-50 transition flex items-center gap-2"
        >
          {loading ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              Downloading...
            </>
          ) : (
            "Load Frame Data"
          )}
        </button>
        <button
          onClick={handleTestMode}
          className="px-4 py-2 rounded-lg bg-purple-600/20 text-purple-400 text-sm font-medium
                     hover:bg-purple-600/40 transition flex items-center gap-2"
          title="Load synthetic test data to verify renderer"
        >
          <FlaskConical className="w-4 h-4" />
          Test Mode
        </button>
      </div>
      {error && (
        <p className="text-xs text-[var(--accent-red)]">{error}</p>
      )}
    </div>
  );
}
