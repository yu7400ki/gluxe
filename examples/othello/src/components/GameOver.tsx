import { Text, View } from "gluxe";

import { C } from "../theme";
import { Button } from "./Button";

interface GameOverProps {
  title: string;
  blackCount: number;
  whiteCount: number;
  onRetry: () => void;
  onMenu: () => void;
}

function ScoreDisc({ color, count }: { color: "black" | "white"; count: number }) {
  const isBlack = color === "black";
  return (
    <View style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 8 }}>
      <View
        style={{
          width: 44,
          height: 44,
          borderRadius: 22,
          backgroundColor: isBlack ? C.black : C.white,
          borderWidth: 1,
          borderColor: isBlack ? C.blackRim : C.whiteRim,
          boxShadow: [
            { offsetY: 2, blurRadius: 5, color: "#00000088" },
            {
              offsetY: -2,
              blurRadius: 3,
              spreadRadius: -1,
              color: isBlack ? C.blackHi : "#ffffff",
              inset: true,
            },
          ],
        }}
      />
      <Text style={{ fontSize: 30, fontWeight: "bold", color: C.text }}>{count}</Text>
    </View>
  );
}

/** Translucent end-of-game overlay drawn over the board. */
export function GameOver({ title, blackCount, whiteCount, onRetry, onMenu }: GameOverProps) {
  return (
    <View
      style={{
        position: "absolute",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        borderRadius: 18,
        backgroundColor: "#f5f1e6f0",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: 24,
      }}
    >
      <Text
        style={{
          fontSize: 40,
          fontWeight: "bold",
          color: C.text,
          textAlign: "center",
        }}
      >
        {title}
      </Text>
      <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 40 }}>
        <ScoreDisc color="black" count={blackCount} />
        <Text style={{ fontSize: 24, color: C.textDimmed, fontWeight: "bold" }}>—</Text>
        <ScoreDisc color="white" count={whiteCount} />
      </View>
      <View style={{ display: "flex", flexDirection: "row", gap: 14 }}>
        <Button label="もう一度" onClick={onRetry} variant="primary" />
        <Button label="メニューへ" onClick={onMenu} variant="ghost" />
      </View>
    </View>
  );
}
