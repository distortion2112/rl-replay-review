// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { TimelineScrubber } from "../components/timeline/TimelineScrubber";
import { usePlaybackStore } from "../hooks/usePlayback";
import type { FrameData } from "../types/frames";

// TimelineScrubber is used rather than PlaybackControls: PlaybackControls is the
// "load frame data" prompt and renders nothing once frame data is in the store.

const initialState = usePlaybackStore.getState();

const frameData: FrameData = {
  players: [],
  frames: [0, 1, 2, 3].map((time) => ({ time, ball: null, players: [] })),
  goals: [{ frame: 2, time: 2, scoring_team: 0, scorer_index: null, ball_position: null, last_touches: [] }],
  duration_seconds: 75,
  total_frames: 4,
};

describe("TimelineScrubber", () => {
  beforeEach(() => {
    usePlaybackStore.setState(initialState, true);
  });

  afterEach(() => {
    cleanup();
  });

  it("renders nothing without frame data", () => {
    const { container } = render(<TimelineScrubber />);
    expect(container.innerHTML).toBe("");
  });

  it("renders the scrubber, transport and speed controls with frame data", () => {
    usePlaybackStore.getState().setFrameData(frameData);
    render(<TimelineScrubber />);

    const slider = screen.getByRole("slider") as HTMLInputElement;
    expect(slider.max).toBe("3");
    expect(slider.value).toBe("0");

    for (const speed of ["0.25x", "0.5x", "1x", "2x"]) {
      expect(screen.getByRole("button", { name: speed })).toBeTruthy();
    }
    // Skip back, play/pause, skip forward, plus the four speed buttons.
    expect(screen.getAllByRole("button")).toHaveLength(7);

    expect(screen.getByText("0:00 / 1:15")).toBeTruthy();
    expect(screen.getByTitle("Goal at 0:02")).toBeTruthy();
  });
});
