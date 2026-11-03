import { beforeEach, describe, expect, it } from "vitest";
import { usePlaybackStore } from "../hooks/usePlayback";
import type { FrameData } from "../types/frames";

const initialState = usePlaybackStore.getState();

// Frame times start above 0 and include a goal-replay gap (3.0 -> 8.0).
const TIMES = [1.5, 2.0, 2.5, 3.0, 8.0, 8.5];

const frameData = (): FrameData => ({
  players: [],
  frames: TIMES.map((time) => ({ time, ball: null, players: [] })),
  goals: [],
  duration_seconds: 8.5,
  total_frames: TIMES.length,
});

const store = () => usePlaybackStore.getState();

describe("usePlaybackStore", () => {
  beforeEach(() => {
    usePlaybackStore.setState(initialState, true);
  });

  it("setFrameData resets the index and sets playbackTime to frame 0's time", () => {
    usePlaybackStore.setState({ currentFrameIndex: 4, playbackTime: 8.2, playing: true });
    store().setFrameData(frameData());
    expect(store().currentFrameIndex).toBe(0);
    expect(store().playbackTime).toBe(1.5);
    expect(store().playing).toBe(false);
  });

  it("setFrameIndex syncs playbackTime to the frame's time", () => {
    store().setFrameData(frameData());
    store().setFrameIndex(3);
    expect(store().currentFrameIndex).toBe(3);
    expect(store().playbackTime).toBe(3.0);
  });

  it("setFrameIndex clamps to the frame range", () => {
    store().setFrameData(frameData());
    store().setFrameIndex(99);
    expect(store().currentFrameIndex).toBe(5);
    expect(store().playbackTime).toBe(8.5);
    store().setFrameIndex(-4);
    expect(store().currentFrameIndex).toBe(0);
    expect(store().playbackTime).toBe(1.5);
  });

  it("seekToTime lands on the first frame at or after the time and syncs playbackTime", () => {
    store().setFrameData(frameData());
    store().seekToTime(2.5);
    expect(store().currentFrameIndex).toBe(2);
    expect(store().playbackTime).toBe(2.5);
    store().seekToTime(5.0); // inside the goal-replay gap
    expect(store().currentFrameIndex).toBe(4);
    expect(store().playbackTime).toBe(8.0);
  });

  it("advancePlayback sets both the index and the continuous time", () => {
    store().setFrameData(frameData());
    store().advancePlayback(2, 2.73);
    expect(store().currentFrameIndex).toBe(2);
    expect(store().playbackTime).toBe(2.73);
  });
});
