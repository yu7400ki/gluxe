import { Text, View } from "@gluxe/react";
import { ScrollArea } from "@gluxe/ui";
import React from "react";

import { theme } from "../theme";
import { Section } from "../ui-kit";

const PARAGRAPHS = [
  "A ScrollArea pairs a clipped, scrollable viewport with a native scrollbar thumb. The library ships behaviour only — the track and thumb you see here are styled entirely by this example.",
  "The viewport sets overflow so the runtime tracks scroll position; the scrollbar's thumb follows it natively, painted inside the track div you style.",
  "Scroll with the mouse wheel inside the box. The thumb on the right reflects how far through the content you are, and you can drag it directly.",
  "Because the thumb is painted natively, there is no z-index juggling: position the track absolutely along an edge of the (relative) root and the thumb takes care of itself.",
  "Headless components keep styling in your hands — swap the colours below for your own palette and the scrollbar follows along.",
];

const COLUMNS = ["One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight"];

export function ScrollAreaSection(): React.ReactElement {
  return (
    <Section
      title="ScrollArea"
      description="A headless scrollable region with native scrollbar thumbs. The viewport clips and scrolls; the scrollbar's thumb tracks it. Both the track and thumb are styled by this example."
    >
      {/* Vertical scroll. The root is `position: relative` so the track can sit
          absolutely down the right edge; the viewport leaves room for it. */}
      <ScrollArea
        style={{
          position: "relative",
          height: 220,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: theme.border,
          backgroundColor: theme.surfaceHigh,
          overflow: "hidden",
        }}
      >
        <ScrollArea.Viewport
          style={{
            height: "100%",
            display: "flex",
            flexDirection: "column",
            gap: 12,
            padding: 16,
            paddingRight: 24,
          }}
        >
          {PARAGRAPHS.map((text, i) => (
            <Text
              // biome-ignore lint/suspicious/noArrayIndexKey: static content
              key={i}
              style={{ color: theme.textMuted, fontSize: 13, lineHeight: 1.6 }}
            >
              {text}
            </Text>
          ))}
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar
          style={{
            position: "absolute",
            top: 4,
            right: 4,
            bottom: 4,
            width: 8,
            borderRadius: 4,
            backgroundColor: theme.track,
          }}
        >
          <ScrollArea.Thumb
            style={{
              backgroundColor: theme.accent,
              borderRadius: 4,
              minHeight: 24,
              margin: 1,
              _hover: { backgroundColor: theme.accentBright },
              _active: { backgroundColor: theme.accentDim },
            }}
          />
        </ScrollArea.Scrollbar>
      </ScrollArea>

      {/* Horizontal scroll. The viewport opts into overflowX and turns off the
          default vertical overflow; the scrollbar sits along the bottom edge. */}
      <ScrollArea
        style={{
          position: "relative",
          width: "100%",
          borderRadius: 10,
          borderWidth: 1,
          borderColor: theme.border,
          backgroundColor: theme.surfaceHigh,
          overflow: "hidden",
        }}
      >
        <ScrollArea.Viewport
          style={{
            overflowX: "scroll",
            overflowY: "hidden",
            display: "flex",
            flexDirection: "row",
            gap: 12,
            padding: 16,
            paddingBottom: 24,
          }}
        >
          {COLUMNS.map((label) => (
            <View
              key={label}
              style={{
                flexShrink: 0,
                width: 140,
                height: 80,
                borderRadius: 8,
                backgroundColor: theme.surface,
                borderWidth: 1,
                borderColor: theme.borderHigh,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              <Text style={{ color: theme.text, fontSize: 14, fontWeight: 500 }}>{label}</Text>
            </View>
          ))}
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar
          orientation="horizontal"
          style={{
            position: "absolute",
            left: 4,
            right: 4,
            bottom: 4,
            height: 8,
            borderRadius: 4,
            backgroundColor: theme.track,
          }}
        >
          <ScrollArea.Thumb
            style={{
              backgroundColor: theme.accent,
              borderRadius: 4,
              minWidth: 24,
              margin: 1,
              _hover: { backgroundColor: theme.accentBright },
              _active: { backgroundColor: theme.accentDim },
            }}
          />
        </ScrollArea.Scrollbar>
      </ScrollArea>
    </Section>
  );
}
