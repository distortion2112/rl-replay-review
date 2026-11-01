import type { FrameData, GameFrame, FramePlayer } from "../types/frames";

/**
 * Generate synthetic frame data with players moving in known patterns.
 * Blue team players orbit on the blue half, orange on the orange half.
 * Ball moves back and forth along the center.
 */
export function generateTestFrameData(): FrameData {
  const players: FramePlayer[] = [
    { index: 0, name: "TestBlue1", team: 0 },
    { index: 1, name: "TestBlue2", team: 0 },
    { index: 2, name: "TestOrange1", team: 1 },
    { index: 3, name: "TestOrange2", team: 1 },
  ];

  const totalSeconds = 60;
  const fps = 30;
  const frames: GameFrame[] = [];

  for (let i = 0; i < totalSeconds * fps; i++) {
    const t = i / fps;
    const angle = (t / 10) * Math.PI * 2;

    frames.push({
      time: t,
      ball: {
        x: Math.sin(angle * 2) * 2000,
        y: Math.sin(angle) * 3000,
        z: 100,
      },
      players: [
        {
          player_index: 0,
          position: {
            x: Math.cos(angle) * 1500 - 1000,
            y: -2500 + Math.sin(angle) * 1000,
            z: 17,
          },
          boost_amount: Math.round((Math.sin(t) * 0.5 + 0.5) * 255),
        },
        {
          player_index: 1,
          position: {
            x: Math.cos(angle + 2) * 1500 + 1000,
            y: -2000 + Math.sin(angle + 2) * 800,
            z: 17,
          },
          boost_amount: Math.round((Math.cos(t * 0.7) * 0.5 + 0.5) * 255),
        },
        {
          player_index: 2,
          position: {
            x: Math.cos(angle + 1) * 1500 - 500,
            y: 2500 + Math.sin(angle + 1) * 1000,
            z: 17,
          },
          boost_amount: Math.round((Math.sin(t * 1.3) * 0.5 + 0.5) * 255),
        },
        {
          player_index: 3,
          position: {
            x: Math.cos(angle + 3) * 1500 + 500,
            y: 2000 + Math.sin(angle + 3) * 800,
            z: 17,
          },
          boost_amount: Math.round((Math.cos(t * 0.5) * 0.5 + 0.5) * 255),
        },
      ],
    });
  }

  return {
    players,
    frames,
    goals: [
      { frame: 300, time: 10 },
      { frame: 900, time: 30 },
    ],
    duration_seconds: totalSeconds,
    total_frames: frames.length,
  };
}
