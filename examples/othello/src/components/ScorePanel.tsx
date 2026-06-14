import { Text, View } from "gluxe";

import { TurnColor } from "../othello";
import { C } from "../theme";

interface PlayerCardProps {
  color: TurnColor;
  name: string;
  count: number;
  active: boolean;
  /** Show the "思考中…" CPU indicator (with animated dots). */
  thinking?: boolean;
  /** Animated dot count 0..3 for the thinking indicator. */
  dots?: number;
}

function PlayerCard({ color, name, count, active, thinking, dots = 0 }: PlayerCardProps) {
  const isBlack = color === "black";
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "row",
        alignItems: "center",
        gap: 12,
        flex: 1,
        paddingX: 16,
        paddingY: 10,
        borderRadius: 14,
        backgroundColor: active ? C.panelHigh : C.panel,
        borderWidth: 1.5,
        borderColor: active ? C.selBorder : C.panelBorder,
      }}
    >
      {/* Disc icon */}
      <View
        style={{
          width: 30,
          height: 30,
          borderRadius: 15,
          backgroundColor: isBlack ? C.black : C.white,
          borderWidth: 1,
          borderColor: isBlack ? C.blackRim : C.whiteRim,
          boxShadow: [
            { offsetY: 1, blurRadius: 3, color: "#00000077" },
            {
              offsetY: -1,
              blurRadius: 2,
              spreadRadius: -1,
              color: isBlack ? C.blackHi : "#ffffff",
              inset: true,
            },
          ],
        }}
      />
      <View style={{ display: "flex", flexDirection: "column", gap: 1, flex: 1 }}>
        <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 6 }}>
          {/* Small gold dot = whose turn it is (echoes the last-move ring on the board) */}
          {active && (
            <View style={{ width: 6, height: 6, borderRadius: 3, backgroundColor: C.gold }} />
          )}
          <Text
            style={{ fontSize: 14, fontWeight: "bold", color: active ? C.text : C.textSecondary }}
          >
            {name}
          </Text>
          {thinking && (
            <Text style={{ fontSize: 11, color: C.textSecondary }}>思考中{".".repeat(dots)}</Text>
          )}
        </View>
        <Text style={{ fontSize: 26, fontWeight: "bold", color: C.text, lineHeight: 1 }}>
          {count}
        </Text>
      </View>
    </View>
  );
}

interface ScorePanelProps {
  turn: TurnColor;
  blackCount: number;
  whiteCount: number;
  blackName: string;
  whiteName: string;
  ended: boolean;
  /** Which colour the CPU is playing, if any. */
  cpuColor: TurnColor | null;
  thinking: boolean;
  dots: number;
}

export function ScorePanel({
  turn,
  blackCount,
  whiteCount,
  blackName,
  whiteName,
  ended,
  cpuColor,
  thinking,
  dots,
}: ScorePanelProps) {
  return (
    <View style={{ display: "flex", flexDirection: "row", gap: 12, width: "100%" }}>
      <PlayerCard
        color="black"
        name={blackName}
        count={blackCount}
        active={!ended && turn === "black"}
        thinking={thinking && cpuColor === "black"}
        dots={dots}
      />
      <PlayerCard
        color="white"
        name={whiteName}
        count={whiteCount}
        active={!ended && turn === "white"}
        thinking={thinking && cpuColor === "white"}
        dots={dots}
      />
    </View>
  );
}
