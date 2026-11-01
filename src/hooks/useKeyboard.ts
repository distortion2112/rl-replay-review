import { useEffect } from "react";
import { usePlaybackStore } from "./usePlayback";
import { useReplayStore } from "./useReplay";

/**
 * Global keyboard shortcuts:
 *
 * Space        - play/pause
 * Left/Right   - step 1 frame
 * Shift+Left/Right - step 10 frames
 * Ctrl+Left/Right  - step 100 frames (big jump)
 * Home/End     - jump to start/end
 * [ / ]        - decrease/increase playback speed
 * 1-4          - select player by index (within current team view)
 * Tab          - cycle selected player
 */
export function useKeyboard() {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Don't capture when typing in an input
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;

      const pb = usePlaybackStore.getState();
      const rs = useReplayStore.getState();
      const hasFrames = !!pb.frameData;

      switch (e.code) {
        case "Space": {
          if (!hasFrames) return;
          e.preventDefault();
          pb.togglePlay();
          break;
        }

        case "ArrowLeft": {
          if (!hasFrames) return;
          e.preventDefault();
          if (pb.playing) pb.pause();
          const stepL = e.ctrlKey ? 100 : e.shiftKey ? 10 : 1;
          pb.setFrameIndex(pb.currentFrameIndex - stepL);
          break;
        }

        case "ArrowRight": {
          if (!hasFrames) return;
          e.preventDefault();
          if (pb.playing) pb.pause();
          const stepR = e.ctrlKey ? 100 : e.shiftKey ? 10 : 1;
          pb.setFrameIndex(pb.currentFrameIndex + stepR);
          break;
        }

        case "ArrowUp":
        case "ArrowDown": {
          // Scroll the middle column
          const mid = document.querySelector("[data-scroll-middle]") as HTMLElement | null;
          if (!mid) return;
          e.preventDefault();
          const amount = e.shiftKey ? 300 : 100;
          mid.scrollBy({ top: e.code === "ArrowUp" ? -amount : amount, behavior: "smooth" });
          break;
        }

        case "Home": {
          if (!hasFrames) return;
          e.preventDefault();
          pb.setFrameIndex(0);
          break;
        }

        case "End": {
          if (!hasFrames || !pb.frameData) return;
          e.preventDefault();
          pb.setFrameIndex(pb.frameData.frames.length - 1);
          break;
        }

        case "BracketLeft": {
          // Decrease speed
          const speeds = [0.25, 0.5, 1, 2];
          const idx = speeds.indexOf(pb.speed);
          if (idx > 0) pb.setSpeed(speeds[idx - 1]);
          break;
        }

        case "BracketRight": {
          // Increase speed
          const speeds = [0.25, 0.5, 1, 2];
          const idx = speeds.indexOf(pb.speed);
          if (idx < speeds.length - 1) pb.setSpeed(speeds[idx + 1]);
          break;
        }

        case "Tab": {
          // Cycle selected player
          e.preventDefault();
          const allPlayers = rs.allPlayers();
          if (allPlayers.length === 0) return;
          const currentIdx = allPlayers.findIndex(
            (p) => p.name === rs.selectedPlayer?.name
          );
          const dir = e.shiftKey ? -1 : 1;
          const nextIdx = (currentIdx + dir + allPlayers.length) % allPlayers.length;
          rs.selectPlayer(allPlayers[nextIdx]);
          break;
        }

        case "Digit1":
        case "Digit2":
        case "Digit3":
        case "Digit4":
        case "Digit5":
        case "Digit6": {
          const idx = parseInt(e.code.replace("Digit", "")) - 1;
          const allPlayers = rs.allPlayers();
          if (idx < allPlayers.length) {
            rs.selectPlayer(allPlayers[idx]);
          }
          break;
        }
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);
}
