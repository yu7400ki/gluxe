import { View } from "@gluxe/react";
import { RadioGroup } from "@gluxe/ui";
import React, { useState } from "react";

import { focusRing, theme } from "../theme";
import { Label, Row, Section } from "../ui-kit";

interface RadioRowProps {
  value: string;
  label: string;
  disabled?: boolean;
}

function RadioRow({ value, label, disabled = false }: RadioRowProps): React.ReactElement {
  return (
    // RadioGroup.Item is the focusable node — roving focus moves between items
    // with the arrow keys (Home / End jump to the ends) and selects as it goes,
    // so the ring belongs here, not on the inner View. Don't pass tabIndex:
    // roving focus owns the Tab order. A little padding keeps the ring clear of
    // the dot and label.
    <RadioGroup.Item
      value={value}
      disabled={disabled}
      style={{ padding: 4, borderRadius: 8, ...focusRing(8) }}
    >
      {({ checked }) => (
        <View style={{ opacity: disabled ? 0.45 : 1 }}>
          <Row>
            <View
              style={{
                display: "flex",
                width: 20,
                height: 20,
                borderRadius: 999,
                borderWidth: 2,
                borderColor: checked ? theme.accent : theme.borderHigh,
                alignItems: "center",
                justifyContent: "center",
                transition: { property: "all", duration: 140, easing: "ease-out" },
              }}
            >
              <RadioGroup.Indicator>
                {() => (
                  <View
                    style={{
                      width: 10,
                      height: 10,
                      borderRadius: 999,
                      backgroundColor: theme.accent,
                    }}
                  />
                )}
              </RadioGroup.Indicator>
            </View>
            <Label>{disabled ? `${label} (coming soon)` : label}</Label>
          </Row>
        </View>
      )}
    </RadioGroup.Item>
  );
}

export function RadioGroupSection(): React.ReactElement {
  const [plan, setPlan] = useState("pro");

  return (
    <Section
      title="RadioGroup"
      description="A single-select list of options. The outer circle and inner dot are styled by reading checked from render-function children; the border and fill animate on selection. The group is one Tab stop — once focused, the arrow keys move between options (Home / End jump to the ends) and selection follows focus, skipping disabled items."
    >
      <RadioGroup
        value={plan}
        onValueChange={setPlan}
        style={{ display: "flex", flexDirection: "column", gap: 12 }}
      >
        {() => (
          <>
            <RadioRow value="free" label="Free" />
            <RadioRow value="pro" label="Pro" />
            <RadioRow value="team" label="Team" />
            <RadioRow value="enterprise" label="Enterprise" disabled />
          </>
        )}
      </RadioGroup>
    </Section>
  );
}
