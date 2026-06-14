import { Text, View } from "gluxe";
import { useState } from "react";

import { Level, TurnColor } from "../othello";
import { C } from "../theme";
import { Button } from "./Button";

export type Mode = "free" | "cpu";

export interface StartConfig {
  mode: Mode;
  level: Level;
  /** The colour the human plays (CPU mode only). */
  playerColor: TurnColor;
}

interface MenuProps {
  onStart: (config: StartConfig) => void;
}

interface ModeCardProps {
  title: string;
  subtitle: string;
  selected: boolean;
  onClick: () => void;
}

function ModeCard({ title, subtitle, selected, onClick }: ModeCardProps) {
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 6,
        paddingX: 22,
        paddingY: 22,
        borderRadius: 16,
        cursor: "pointer",
        backgroundColor: selected ? C.panelHigh : C.panel,
        borderWidth: 1.5,
        borderColor: selected ? C.selBorder : C.panelBorder,
        boxShadow: [{ offsetY: 4, blurRadius: 12, spreadRadius: -4, color: "#00000022" }],
        _hover: {
          backgroundColor: C.panelHigh,
          borderColor: selected ? C.selBorder : C.hoverBorder,
        },
        _active: { backgroundColor: C.panel },
      }}
      onClick={onClick}
    >
      <Text
        style={{ fontSize: 18, fontWeight: "bold", color: selected ? C.text : C.textSecondary }}
      >
        {title}
      </Text>
      <Text style={{ fontSize: 12, color: C.textSecondary, lineHeight: 1.4, whiteSpace: "nowrap" }}>
        {subtitle}
      </Text>
    </View>
  );
}

interface PillProps {
  label: string;
  selected: boolean;
  onClick: () => void;
}

function Pill({ label, selected, onClick }: PillProps) {
  return (
    <View
      style={{
        paddingX: 18,
        paddingY: 8,
        borderRadius: 999,
        cursor: "pointer",
        backgroundColor: selected ? C.panelHigh : C.panel,
        borderWidth: 1,
        borderColor: selected ? C.selBorder : C.panelBorder,
        _hover: { borderColor: selected ? C.selBorder : C.hoverBorder },
      }}
      onClick={onClick}
    >
      <Text
        style={{
          fontSize: 13,
          fontWeight: "bold",
          color: selected ? C.text : C.textSecondary,
        }}
      >
        {label}
      </Text>
    </View>
  );
}

interface ColorChoiceProps {
  color: TurnColor;
  label: string;
  selected: boolean;
  onClick: () => void;
}

function ColorChoice({ color, label, selected, onClick }: ColorChoiceProps) {
  const isBlack = color === "black";
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "row",
        alignItems: "center",
        gap: 10,
        paddingX: 16,
        paddingY: 10,
        borderRadius: 12,
        cursor: "pointer",
        backgroundColor: selected ? C.panelHigh : C.panel,
        borderWidth: 1.5,
        borderColor: selected ? C.selBorder : C.panelBorder,
        _hover: { borderColor: selected ? C.selBorder : C.hoverBorder },
      }}
      onClick={onClick}
    >
      <View
        style={{
          width: 22,
          height: 22,
          borderRadius: 11,
          backgroundColor: isBlack ? C.black : C.white,
          borderWidth: 1,
          borderColor: isBlack ? C.blackRim : C.whiteRim,
          boxShadow: [{ offsetY: 1, blurRadius: 2, color: "#00000066" }],
        }}
      />
      <Text
        style={{ fontSize: 13, fontWeight: "bold", color: selected ? C.text : C.textSecondary }}
      >
        {label}
      </Text>
    </View>
  );
}

export function Menu({ onStart }: MenuProps) {
  const [mode, setMode] = useState<Mode>("cpu");
  const [level, setLevel] = useState<Level>("normal");
  const [playerColor, setPlayerColor] = useState<TurnColor>("black");

  return (
    <View
      style={{
        position: "relative",
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        alignItems: "center",
        justifyContent: "center",
        paddingX: 40,
        gap: 28,
      }}
    >
      {/* Title */}
      <Text
        style={{
          fontSize: 64,
          fontWeight: "bold",
          color: C.text,
          textAlign: "center",
        }}
      >
        OTHELLO
      </Text>

      {/* Mode cards: 2-column grid (1fr 1fr) */}
      <View
        style={{ display: "grid", gridTemplateColumns: 2, gap: 16, width: "100%", maxWidth: 460 }}
      >
        <ModeCard
          title="ひとりで打つ"
          subtitle="黒も白も自分で打つ自由対局"
          selected={mode === "free"}
          onClick={() => setMode("free")}
        />
        <ModeCard
          title="CPUと対戦"
          subtitle="3段階のつよさから選んで勝負"
          selected={mode === "cpu"}
          onClick={() => setMode("cpu")}
        />
      </View>

      {/* CPU options — fixed-height slot so toggling the mode doesn't shift the layout */}
      <View
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          gap: 16,
          width: "100%",
          maxWidth: 460,
          height: 160,
        }}
      >
        {mode === "cpu" && (
          <>
            <View
              style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 8 }}
            >
              <Text style={{ fontSize: 12, color: C.textDimmed, fontWeight: "bold" }}>つよさ</Text>
              <View style={{ display: "flex", flexDirection: "row", gap: 10 }}>
                <Pill
                  label="やさしい"
                  selected={level === "easy"}
                  onClick={() => setLevel("easy")}
                />
                <Pill
                  label="ふつう"
                  selected={level === "normal"}
                  onClick={() => setLevel("normal")}
                />
                <Pill label="つよい" selected={level === "hard"} onClick={() => setLevel("hard")} />
              </View>
            </View>

            <View
              style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 8 }}
            >
              <Text style={{ fontSize: 12, color: C.textDimmed, fontWeight: "bold" }}>てばん</Text>
              <View style={{ display: "flex", flexDirection: "row", gap: 12 }}>
                <ColorChoice
                  color="black"
                  label="先手・黒"
                  selected={playerColor === "black"}
                  onClick={() => setPlayerColor("black")}
                />
                <ColorChoice
                  color="white"
                  label="後手・白"
                  selected={playerColor === "white"}
                  onClick={() => setPlayerColor("white")}
                />
              </View>
            </View>
          </>
        )}
      </View>

      {/* Start */}
      <Button label="対局開始" large onClick={() => onStart({ mode, level, playerColor })} />
    </View>
  );
}
