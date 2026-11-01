import { Loader2, Film } from "lucide-react";
import { usePlaybackStore } from "../../hooks/usePlayback";
import { useReplayStore } from "../../hooks/useReplay";
import { downloadAndParseReplay } from "../../lib/tauri-commands";

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
      setFrameData(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  if (frameData) return null;

  return (
    <div className="bg-[var(--bg-card)] rounded-lg p-4 text-center space-y-2">
      <Film className="w-8 h-8 text-[var(--text-secondary)] mx-auto" />
      <p className="text-sm text-[var(--text-secondary)]">
        Load frame-by-frame data to see player positions, timeline playback,
        and detailed boost tracking.
      </p>
      <button
        onClick={handleLoad}
        disabled={loading || !replay}
        className="px-4 py-2 rounded-lg bg-[var(--accent-blue)] text-white text-sm font-medium
                   hover:bg-blue-600 disabled:opacity-50 transition flex items-center gap-2 mx-auto"
      >
        {loading ? (
          <>
            <Loader2 className="w-4 h-4 animate-spin" />
            Downloading & parsing...
          </>
        ) : (
          "Load Frame Data"
        )}
      </button>
      {error && (
        <p className="text-xs text-[var(--accent-red)]">{error}</p>
      )}
    </div>
  );
}
