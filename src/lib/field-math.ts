// Rocket League field dimensions in Unreal Units
export const FIELD = {
  WIDTH: 8192, // X axis (side to side)
  LENGTH: 10240, // Y axis (goal to goal)
  HALF_WIDTH: 4096,
  HALF_LENGTH: 5120,
  GOAL_WIDTH: 1786,
  GOAL_DEPTH: 880,
  CORNER_RADIUS: 1152, // rounded corner radius in UU
  CEILING_HEIGHT: 2044, // ceiling height
  WALL_HEIGHT: 2044,
  BALL_RADIUS: 92.75,
};

export interface BoostPad {
  x: number;
  y: number;
  big: boolean;
}

// All 34 boost pad positions (mirrored field, Y=0 is center)
export const BOOST_PADS: BoostPad[] = [
  // Big pads (6)
  { x: -3072, y: -4096, big: true },
  { x: 3072, y: -4096, big: true },
  { x: -3584, y: 0, big: true },
  { x: 3584, y: 0, big: true },
  { x: -3072, y: 4096, big: true },
  { x: 3072, y: 4096, big: true },
  // Small pads (28)
  { x: 0, y: -4240, big: false },
  { x: -1792, y: -4184, big: false },
  { x: 1792, y: -4184, big: false },
  { x: -940, y: -3308, big: false },
  { x: 940, y: -3308, big: false },
  { x: 0, y: -2816, big: false },
  { x: -3584, y: -2484, big: false },
  { x: 3584, y: -2484, big: false },
  { x: -1788, y: -2300, big: false },
  { x: 1788, y: -2300, big: false },
  { x: -940, y: -1152, big: false },
  { x: 940, y: -1152, big: false },
  { x: 0, y: -1024, big: false },
  { x: -1024, y: 0, big: false },
  { x: 1024, y: 0, big: false },
  { x: 0, y: 1024, big: false },
  { x: -940, y: 1152, big: false },
  { x: 940, y: 1152, big: false },
  { x: -1788, y: 2300, big: false },
  { x: 1788, y: 2300, big: false },
  { x: -3584, y: 2484, big: false },
  { x: 3584, y: 2484, big: false },
  { x: 0, y: 2816, big: false },
  { x: -940, y: 3308, big: false },
  { x: 940, y: 3308, big: false },
  { x: -1792, y: 4184, big: false },
  { x: 1792, y: 4184, big: false },
  { x: 0, y: 4240, big: false },
];

export function fieldToCanvas(
  fieldX: number,
  fieldY: number,
  canvasWidth: number,
  canvasHeight: number,
  padding = 20
): { x: number; y: number } {
  const drawW = canvasWidth - padding * 2;
  const drawH = canvasHeight - padding * 2;
  const x = padding + ((fieldX + FIELD.HALF_WIDTH) / FIELD.WIDTH) * drawW;
  const y = padding + ((fieldY + FIELD.HALF_LENGTH) / FIELD.LENGTH) * drawH;
  return { x, y };
}
