// Checkbox section: a "select toppings" checklist demonstrating the tri-state
// Checkbox from @gluxe/ui. The parent "Select all" checkbox computes its state
// from the three children: true (all), false (none), or "indeterminate" (some).
//
// The headless Checkbox supplies click handling and state cycling. Everything
// visual — the box border, fill colour, glyph — comes from the render-function
// children reading `checked` from state.

import { Text, View } from "@gluxe/react";
import { Checkbox, type CheckedState } from "@gluxe/ui";
import React, { useState } from "react";

import { focusRing, theme } from "../theme";
import { Label, Row, Section } from "../ui-kit";

function CheckRow({
  checked,
  onChange,
  label,
}: {
  checked: CheckedState;
  onChange: (next: boolean) => void;
  label: string;
}): React.ReactElement {
  return (
    <Row>
      {/* Checkbox is the focusable node — ring goes here, matching the box radius. */}
      <Checkbox checked={checked} onCheckedChange={onChange} style={focusRing(8)}>
        {({ checked: cs }) => (
          <View
            style={{
              display: "flex",
              width: 22,
              height: 22,
              borderRadius: 6,
              borderWidth: 2,
              borderColor: cs !== false ? theme.accent : theme.borderHigh,
              backgroundColor: cs !== false ? theme.accent : "transparent",
              alignItems: "center",
              justifyContent: "center",
              transition: { property: "all", duration: 140, easing: "ease-out" },
            }}
          >
            <Checkbox.Indicator>
              {({ checked: ci }) => (
                <Text
                  style={{
                    color: "#ffffff",
                    fontSize: 13,
                    fontWeight: "bold",
                    lineHeight: 1,
                  }}
                >
                  {ci === "indeterminate" ? "–" : "✓"}
                </Text>
              )}
            </Checkbox.Indicator>
          </View>
        )}
      </Checkbox>
      <Label>{label}</Label>
    </Row>
  );
}

export function CheckboxSection(): React.ReactElement {
  const [pepperoni, setPepperoni] = useState(true);
  const [mushroom, setMushroom] = useState(false);
  const [onion, setOnion] = useState(false);

  const allSelected = pepperoni && mushroom && onion;
  const noneSelected = !pepperoni && !mushroom && !onion;
  const parentChecked: CheckedState = allSelected ? true : noneSelected ? false : "indeterminate";

  function handleSelectAll(next: boolean): void {
    setPepperoni(next);
    setMushroom(next);
    setOnion(next);
  }

  return (
    <Section
      title="Checkbox"
      description="A tri-state checkbox control. The parent shows an indeterminate (–) state when only some children are selected. Click or press Space / Enter (after tabbing to it) to select all; do it again to clear all."
    >
      <CheckRow checked={parentChecked} onChange={handleSelectAll} label="Select all toppings" />
      <View style={{ display: "flex", flexDirection: "column", gap: 10, marginLeft: 28 }}>
        <CheckRow checked={pepperoni} onChange={setPepperoni} label="Pepperoni" />
        <CheckRow checked={mushroom} onChange={setMushroom} label="Mushroom" />
        <CheckRow checked={onion} onChange={setOnion} label="Onion" />
      </View>
    </Section>
  );
}
