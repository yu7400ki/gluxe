import { View } from "@gluxe/react";

import { C, CELL } from "../theme";

interface DiscProps {
  /** 1 = black, 2 = white. */
  color: 1 | 2;
  /** Disc diameter in px (animated). */
  size?: number;
  /** Horizontal squash factor 0..1 (for the flip; 1 = full width). */
  scaleX?: number;
  /** Show the gold "last move" ring marker. */
  lastMove?: boolean;
}

/**
 * A single Othello disc rendered as a layered circle.
 *
 * 3D sheen is faked with multiple box-shadow layers: a drop shadow for depth,
 * plus inset highlight/shade to suggest a glossy curved surface. The flip
 * animation squashes `scaleX` toward 0 around the disc centre; the caller swaps
 * `color` at the midpoint so the disc appears to rotate edge-on.
 */
export function Disc({ color, size = CELL - 12, scaleX = 1, lastMove = false }: DiscProps) {
  const isBlack = color === 1;
  const width = Math.max(0, size * scaleX);

  return (
    <View
      style={{
        position: "absolute",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <View
        style={{
          width,
          height: size,
          borderRadius: size / 2,
          backgroundColor: isBlack ? C.black : C.white,
          borderWidth: 1,
          borderColor: isBlack ? C.blackRim : C.whiteRim,
          boxShadow: isBlack
            ? [
                { offsetY: 2, blurRadius: 4, color: "#00000066" },
                { offsetY: -2, blurRadius: 3, spreadRadius: -1, color: C.blackHi, inset: true },
                { offsetY: 3, blurRadius: 4, spreadRadius: -2, color: "#000000aa", inset: true },
              ]
            : [
                { offsetY: 2, blurRadius: 4, color: "#00000055" },
                { offsetY: -2, blurRadius: 3, spreadRadius: -1, color: "#ffffff", inset: true },
                { offsetY: 3, blurRadius: 4, spreadRadius: -2, color: C.whiteShade, inset: true },
              ],
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        {lastMove && scaleX > 0.85 && (
          <View
            style={{
              width: size * 0.32,
              height: size * 0.32,
              borderRadius: size * 0.16,
              borderWidth: 2,
              borderColor: C.gold,
              backgroundColor: "transparent",
              boxShadow: { blurRadius: 6, color: C.goldGlow },
            }}
          />
        )}
      </View>
    </View>
  );
}
