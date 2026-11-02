import { create } from "zustand";
import type { FrameData, GameFrame } from "../types/frames";

interface PlaybackStore {
  frameData: FrameData | null;
  currentFrameIndex: number;
  /** Continuous replay time in seconds; between frame timestamps during playback. */
  playbackTime: number;
  playing: boolean;
  speed: number;
  loading: boolean;
  error: string | null;

  setFrameData: (data: FrameData | null) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  setFrameIndex: (index: number) => void;
  /** Advance playback to a continuous time; `index` is the latest frame at or before it. */
  advancePlayback: (index: number, time: number) => void;
  play: () => void;
  pause: () => void;
  togglePlay: () => void;
  setSpeed: (speed: number) => void;
  currentFrame: () => GameFrame | null;
  seekToTime: (time: number) => void;
}

export const usePlaybackStore = create<PlaybackStore>((set, get) => ({
  frameData: null,
  currentFrameIndex: 0,
  playbackTime: 0,
  playing: false,
  speed: 1,
  loading: false,
  error: null,

  setFrameData: (data) =>
    set({ frameData: data, currentFrameIndex: 0, playbackTime: data?.frames[0]?.time ?? 0, playing: false }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),
  setFrameIndex: (index) => {
    const data = get().frameData;
    if (!data) return;
    const clamped = Math.max(0, Math.min(index, data.frames.length - 1));
    set({ currentFrameIndex: clamped, playbackTime: data.frames[clamped]?.time ?? 0 });
  },
  advancePlayback: (index, time) => set({ currentFrameIndex: index, playbackTime: time }),
  play: () => set({ playing: true }),
  pause: () => set({ playing: false }),
  togglePlay: () => set((s) => ({ playing: !s.playing })),
  setSpeed: (speed) => set({ speed }),

  currentFrame: () => {
    const { frameData, currentFrameIndex } = get();
    return frameData?.frames[currentFrameIndex] ?? null;
  },

  seekToTime: (time: number) => {
    const data = get().frameData;
    if (!data) return;
    // Binary search for closest frame
    let lo = 0;
    let hi = data.frames.length - 1;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (data.frames[mid].time < time) {
        lo = mid + 1;
      } else {
        hi = mid;
      }
    }
    set({ currentFrameIndex: lo, playbackTime: data.frames[lo]?.time ?? 0 });
  },
}));
