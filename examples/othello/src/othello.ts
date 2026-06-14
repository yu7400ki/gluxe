// Typed invoke wrappers around the Rust `othello` plugin.

import { invoke } from "@gluxe/react";

/** Cell occupancy: 0 = empty, 1 = black, 2 = white. */
export type Cell = 0 | 1 | 2;

export type TurnColor = "black" | "white";
export type Level = "easy" | "normal" | "hard";

/** Opaque board snapshot sent back to Rust (hex-encoded bitboards). */
export interface BoardDto {
  black: string;
  white: string;
  turn: TurnColor;
  passed: boolean;
  ended: boolean;
}

/** Full state description returned by the Rust side. */
export interface StateView {
  black: string;
  white: string;
  turn: TurnColor;
  passed: boolean;
  ended: boolean;
  cells: Cell[];
  legal: number[];
  blackCount: number;
  whiteCount: number;
  /** Index of the most recently played disc, or null. */
  lastMove: number | null;
  /** Indices of discs that flipped colour on the last move. */
  flipped: number[];
}

export interface CpuResult {
  move: number | null;
  state: StateView;
}

/** Extract the persistable DTO from a full state view. */
export function toDto(s: StateView): BoardDto {
  return {
    black: s.black,
    white: s.white,
    turn: s.turn,
    passed: s.passed,
    ended: s.ended,
  };
}

export function newGame(): Promise<StateView> {
  return invoke<StateView>("othello|new", {});
}

export function play(board: BoardDto, index: number): Promise<StateView> {
  return invoke<StateView>("othello|play", { board, index });
}

export function cpuMove(board: BoardDto, level: Level): Promise<CpuResult> {
  return invoke<CpuResult>("othello|cpuMove", { board, level });
}
