// Light premium Othello theme — warm ivory surfaces, the dark green board
// sits on them like a real table.

export const C = {
  // Backgrounds
  bg: "#f1ecdf",
  panel: "#faf7ef",
  panelHigh: "#efe9d9",
  panelBorder: "#ddd4c0",
  // Neutral selection/hover borders (gold is reserved for rare accents)
  selBorder: "#b3a685",
  hoverBorder: "#cdc3a9",

  // Text
  text: "#3b3427",
  textSecondary: "#857c68",
  textDimmed: "#b4ab94",

  // Accent (warm gold)
  gold: "#c2973f",
  goldBright: "#d2a851",
  goldDim: "#a8823a",
  goldGlow: "#c2973f66",

  // Board (green felt + wooden frame)
  feltLight: "#1c7a52",
  feltDark: "#176546",
  feltLine: "#0f4d35",
  frameOuter: "#3a2d22",
  frameInner: "#5a432e",
  frameHighlight: "#6e5238",

  // Discs
  black: "#1c1c1e",
  blackRim: "#3a3a3e",
  blackHi: "#48484e",
  white: "#f4f1e8",
  whiteRim: "#cdc6b4",
  whiteShade: "#d8d2c2",
};

// Board geometry (logical px)
export const CELL = 60;
export const CELL_GAP = 3;
export const BOARD_PADDING = 6;
export const FRAME_WIDTH = 14;
// Inner playfield (8 cells + 7 gaps + padding both sides)
export const PLAYFIELD = CELL * 8 + CELL_GAP * 7 + BOARD_PADDING * 2;
export const BOARD_SIZE = PLAYFIELD + FRAME_WIDTH * 2;

/** Pixel offset (within the playfield) of grid cell index `i` (row or column). */
export const cellOffset = (i: number) => BOARD_PADDING + i * (CELL + CELL_GAP);

// Animation timings (ms)
export const FLIP_MS = 220;
export const FLIP_STAGGER_MS = 70;
export const POP_MS = 160;

/** Chebyshev distance between two cell indices (ring distance from last move). */
export function chebyshev(a: number, b: number): number {
  const ar = Math.floor(a / 8);
  const ac = a % 8;
  const br = Math.floor(b / 8);
  const bc = b % 8;
  return Math.max(Math.abs(ar - br), Math.abs(ac - bc));
}
