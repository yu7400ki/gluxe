import { View } from "@gluxe/react";

import { Cell, StateView, TurnColor } from "../othello";
import {
  BOARD_PADDING,
  BOARD_SIZE,
  C,
  CELL,
  CELL_GAP,
  FLIP_MS,
  FLIP_STAGGER_MS,
  FRAME_WIDTH,
  PLAYFIELD,
  POP_MS,
  cellOffset,
  chebyshev,
} from "../theme";
import { Disc } from "./Disc";

interface BoardViewProps {
  state: StateView;
  /** Cell colours before the last move (for showing the pre-flip face). */
  prevCells: Cell[] | null;
  /** Elapsed ms since the flip animation started (0 when idle). */
  animT: number;
  /** Whether an animation is currently running. */
  animating: boolean;
  /** Click handler — receives the cell index. Ignored when input is locked. */
  onCellClick: (index: number) => void;
  /** When true, legal-move hints are hidden (e.g. CPU thinking / game over). */
  hideHints?: boolean;
}

const STAR_POINTS = [
  [2, 2],
  [2, 6],
  [6, 2],
  [6, 6],
];

/** Resolve a disc's rendered colour + scaleX for the current animation frame. */
function discFrame(
  index: number,
  current: Cell,
  prev: Cell | undefined,
  lastMove: number | null,
  flipped: number[],
  animT: number,
  animating: boolean,
): { color: 1 | 2; scaleX: number; size: number } | null {
  const fullSize = CELL - 12;
  if (current === 0) return null;

  if (!animating) {
    return { color: current as 1 | 2, scaleX: 1, size: fullSize };
  }

  // Freshly placed disc pops in.
  if (index === lastMove) {
    const p = Math.min(1, animT / POP_MS);
    // Ease-out overshoot-ish: grow from 0 to full.
    const eased = 1 - (1 - p) * (1 - p);
    return { color: current as 1 | 2, scaleX: 1, size: fullSize * eased };
  }

  // Flipped disc: rotate edge-on with a stagger based on ring distance.
  if (flipped.includes(index) && lastMove !== null) {
    const delay = chebyshev(index, lastMove) * FLIP_STAGGER_MS;
    const te = animT - delay;
    if (te <= 0) {
      // Not started yet — show the old face.
      return { color: (prev ?? current) as 1 | 2, scaleX: 1, size: fullSize };
    }
    if (te >= FLIP_MS) {
      return { color: current as 1 | 2, scaleX: 1, size: fullSize };
    }
    const p = te / FLIP_MS;
    const scaleX = Math.abs(Math.cos(p * Math.PI));
    const color = (p < 0.5 ? (prev ?? current) : current) as 1 | 2;
    return { color, scaleX, size: fullSize };
  }

  // Settled disc.
  return { color: current as 1 | 2, scaleX: 1, size: fullSize };
}

export function BoardView({
  state,
  prevCells,
  animT,
  animating,
  onCellClick,
  hideHints,
}: BoardViewProps) {
  const hintColor: TurnColor = state.turn;
  const legalSet = new Set(state.legal);

  return (
    <View
      style={{
        position: "relative",
        width: BOARD_SIZE,
        height: BOARD_SIZE,
        borderRadius: 18,
        backgroundColor: C.frameInner,
        borderWidth: 1,
        borderColor: C.frameHighlight,
        boxShadow: [
          { offsetY: 12, blurRadius: 32, spreadRadius: -6, color: "#00000055" },
          { offsetY: 2, blurRadius: 1, color: C.frameHighlight, inset: true },
          { offsetY: -3, blurRadius: 6, spreadRadius: -2, color: "#00000088", inset: true },
        ],
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      {/* Wooden frame outline */}
      <View
        style={{
          position: "absolute",
          top: 4,
          left: 4,
          right: 4,
          bottom: 4,
          borderRadius: 14,
          borderWidth: 2,
          borderColor: C.frameOuter,
          backgroundColor: "transparent",
        }}
      />

      {/* Green felt playfield */}
      <View
        style={{
          position: "relative",
          width: PLAYFIELD,
          height: PLAYFIELD,
          borderRadius: 8,
          backgroundColor: C.feltDark,
          borderWidth: 1,
          borderColor: C.feltLine,
          boxShadow: {
            offsetY: 2,
            blurRadius: 10,
            spreadRadius: -2,
            color: "#00000099",
            inset: true,
          },
        }}
      >
        {/* Grid cells */}
        {Array.from({ length: 64 }, (_, i) => {
          const row = Math.floor(i / 8);
          const col = i % 8;
          const checker = (row + col) % 2 === 0;
          const isLegal = !hideHints && legalSet.has(i);
          const frame = discFrame(
            i,
            state.cells[i],
            prevCells?.[i],
            state.lastMove,
            state.flipped,
            animT,
            animating,
          );
          return (
            <View
              key={i}
              style={{
                position: "absolute",
                top: cellOffset(row),
                left: cellOffset(col),
                width: CELL,
                height: CELL,
                borderRadius: 4,
                backgroundColor: checker ? C.feltLight : C.feltDark,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                cursor: isLegal ? "pointer" : "default",
                ...(isLegal
                  ? { _hover: { backgroundColor: checker ? "#249a68" : "#1d8a5c" } }
                  : {}),
              }}
              onClick={isLegal ? () => onCellClick(i) : undefined}
            >
              {/* Legal-move hint dot */}
              {isLegal && frame === null && (
                <View
                  style={{
                    width: CELL * 0.26,
                    height: CELL * 0.26,
                    borderRadius: CELL * 0.13,
                    backgroundColor: hintColor === "black" ? "#00000055" : "#ffffff66",
                    borderWidth: 1,
                    borderColor: hintColor === "black" ? "#00000033" : "#ffffff44",
                  }}
                />
              )}
              {/* Disc */}
              {frame && (
                <Disc
                  color={frame.color}
                  size={frame.size}
                  scaleX={frame.scaleX}
                  lastMove={!animating && i === state.lastMove}
                />
              )}
            </View>
          );
        })}

        {/* Star points at the 2-2 intersections */}
        {STAR_POINTS.map(([r, c], k) => (
          <View
            key={`star-${k}`}
            style={{
              position: "absolute",
              top: cellOffset(r) - CELL_GAP / 2 - 2,
              left: cellOffset(c) - CELL_GAP / 2 - 2,
              width: 5,
              height: 5,
              borderRadius: 3,
              backgroundColor: "#0c4730",
            }}
          />
        ))}
      </View>
    </View>
  );
}
