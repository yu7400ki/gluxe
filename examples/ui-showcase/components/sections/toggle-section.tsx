// Text-formatting toolbar demo built on the headless @gluxe/ui `Toggle`.
//
// Toggle supplies behaviour (pressed state + click handler) only. Every visual
// detail — background, text colour, hover tint, transition — comes from the
// render-function children below, which read `pressed` from state.

import { Text, View } from "@gluxe/react";
import { Toggle } from "@gluxe/ui";
import React, { useState } from "react";

import { focusRing, theme } from "../theme";
import { Row, Section } from "../ui-kit";

interface FormatButtonProps {
  pressed: boolean;
  onPressedChange: (next: boolean) => void;
  label: string;
  fontWeight?: "bold" | "normal";
  fontStyle?: "italic" | "normal";
  textDecorationLine?: "underline" | "none";
}

function FormatButton({
  pressed,
  onPressedChange,
  label,
  fontWeight = "normal",
  fontStyle = "normal",
  textDecorationLine = "none",
}: FormatButtonProps): React.ReactElement {
  return (
    // Toggle is itself the focusable node, so the focus ring goes here — not on
    // the inner View. Tab reaches it; Space / Enter flip the pressed state.
    <Toggle pressed={pressed} onPressedChange={onPressedChange} style={focusRing(8)}>
      {({ pressed }) => (
        <View
          style={{
            display: "flex",
            width: 40,
            height: 36,
            borderRadius: 8,
            alignItems: "center",
            justifyContent: "center",
            backgroundColor: pressed ? theme.accent : theme.surfaceHigh,
            transition: { property: "all", duration: 140, easing: "ease-out" },
            _hover: {
              backgroundColor: pressed ? theme.accent : theme.borderHigh,
            },
          }}
        >
          <Text
            style={{
              color: pressed ? theme.accentText : theme.text,
              fontSize: 15,
              fontWeight,
              fontStyle,
              textDecorationLine,
              transition: { property: "all", duration: 140, easing: "ease-out" },
            }}
          >
            {label}
          </Text>
        </View>
      )}
    </Toggle>
  );
}

export function ToggleSection(): React.ReactElement {
  const [bold, setBold] = useState(false);
  const [italic, setItalic] = useState(false);
  const [underline, setUnderline] = useState(false);

  return (
    <Section
      title="Toggle"
      description="A headless pressable with boolean pressed state. The button shape, active colour, and hover tint are styled here via render-function children that read pressed. Tab to focus a button and press Space or Enter to toggle it."
    >
      <Row>
        <FormatButton pressed={bold} onPressedChange={setBold} label="B" fontWeight="bold" />
        <FormatButton pressed={italic} onPressedChange={setItalic} label="I" fontStyle="italic" />
        <FormatButton
          pressed={underline}
          onPressedChange={setUnderline}
          label="U"
          textDecorationLine="underline"
        />
      </Row>
      <Row>
        <Text
          style={{
            color: theme.textMuted,
            fontSize: 14,
            lineHeight: 1.5,
            fontWeight: bold ? "bold" : "normal",
            fontStyle: italic ? "italic" : "normal",
            textDecorationLine: underline ? "underline" : "none",
          }}
        >
          The quick brown fox jumps over the lazy dog.
        </Text>
      </Row>
    </Section>
  );
}
