import { Text, View } from "@gluxe/react";
import { Select } from "@gluxe/ui";
import React, { useState } from "react";

import { focusRing, theme } from "../theme";
import { Section } from "../ui-kit";

const FRAMEWORKS: { value: string; label: string }[] = [
  { value: "react", label: "React" },
  { value: "solid", label: "Solid" },
  { value: "svelte", label: "Svelte" },
  { value: "vue", label: "Vue" },
  { value: "angular", label: "Angular (legacy)" },
];

const LABELS = Object.fromEntries(FRAMEWORKS.map((f) => [f.value, f.label]));

export function SelectSection(): React.ReactElement {
  const [value, setValue] = useState<string | undefined>(undefined);

  return (
    <Section
      title="Select"
      description="A single-select dropdown. The trigger is the only Tab stop; Enter / Space or Down / Up open the list, the arrow keys move between options (Home / End jump to the ends, disabled options are skipped), Enter selects, and Escape or a click outside closes — all returning focus to the trigger. The floating list is anchored to the trigger and lifts above other content."
    >
      <Select value={value} onValueChange={setValue} loop>
        <Select.Trigger
          style={{
            display: "flex",
            flexDirection: "row",
            alignItems: "center",
            justifyContent: "space-between",
            gap: 10,
            width: 240,
            paddingX: 12,
            paddingY: 9,
            borderRadius: 10,
            borderWidth: 1,
            borderColor: theme.borderHigh,
            backgroundColor: theme.surfaceHigh,
            ...focusRing(10),
          }}
        >
          {({ value: v }) => (
            <>
              <Text
                style={{
                  color: v ? theme.text : theme.textMuted,
                  fontSize: 14,
                }}
              >
                {v ? LABELS[v] : "Select a framework…"}
              </Text>
              <Text
                style={{
                  color: theme.textMuted,
                  fontSize: 12,
                  transition: {
                    property: "all",
                    duration: 140,
                    easing: "ease-out",
                  },
                }}
              >
                ▼
              </Text>
            </>
          )}
        </Select.Trigger>

        <Select.Content
          offset={6}
          style={{
            display: "flex",
            flexDirection: "column",
            gap: 2,
            width: 240,
            padding: 5,
            borderRadius: 12,
            borderWidth: 1,
            borderColor: theme.border,
            backgroundColor: theme.surface,
            boxShadow: "lg",
          }}
        >
          {FRAMEWORKS.map((f) => (
            <Select.Item
              key={f.value}
              value={f.value}
              disabled={f.value === "angular"}
              style={{ borderRadius: 8, ...focusRing(8) }}
            >
              {({ selected, highlighted, disabled }) => (
                <View
                  style={{
                    display: "flex",
                    flexDirection: "row",
                    alignItems: "center",
                    justifyContent: "space-between",
                    gap: 10,
                    paddingX: 10,
                    paddingY: 8,
                    borderRadius: 8,
                    opacity: disabled ? 0.4 : 1,
                    backgroundColor: highlighted ? theme.surfaceHigh : "transparent",
                  }}
                >
                  <Text
                    style={{
                      color: selected ? theme.accentBright : theme.text,
                      fontSize: 14,
                      fontWeight: selected ? "bold" : "normal",
                    }}
                  >
                    {f.label}
                  </Text>
                  <Select.ItemIndicator>
                    <Text style={{ color: theme.accent, fontSize: 13 }}>✓</Text>
                  </Select.ItemIndicator>
                </View>
              )}
            </Select.Item>
          ))}
        </Select.Content>
      </Select>
    </Section>
  );
}
