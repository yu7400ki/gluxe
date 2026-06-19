// Reference section: styled buttons built on the headless @gluxe/ui `Button`.
//
// Button is the simplest part — a stateless focusable pressable with no parts
// and no render-function children. It supplies behaviour only (`onClick` fires
// on a click or on Space / Enter while focused); every visual below is applied
// here. `disabled` is inert and out of the Tab order — we dim it ourselves.

import { Text, View } from "@gluxe/react";
import { Button } from "@gluxe/ui";
import React, { useState } from "react";

import { focusRing, theme } from "../theme";
import { Label, Row, Section } from "../ui-kit";

export function ButtonSection(): React.ReactElement {
  const [count, setCount] = useState(0);

  return (
    <Section
      title="Button"
      description="A stateless focusable pressable. Tab to focus it and click or press Space or Enter to activate; onClick is the single press callback for both. A disabled button is inert and skipped in the Tab order."
    >
      <Row>
        {/* Button is itself the focusable node, so the focus ring goes here. */}
        <Button
          onClick={() => setCount((c) => c + 1)}
          style={{
            display: "flex",
            paddingX: 16,
            paddingY: 9,
            borderRadius: 8,
            alignItems: "center",
            justifyContent: "center",
            backgroundColor: theme.accent,
            transition: { property: "all", duration: 140, easing: "ease-out" },
            _hover: { backgroundColor: theme.accentBright },
            _active: { backgroundColor: theme.accentDim },
            ...focusRing(8),
          }}
        >
          <Text style={{ color: theme.accentText, fontSize: 14, fontWeight: "bold" }}>
            Click me
          </Text>
        </Button>
        <Label>Clicked {count} times</Label>
      </Row>
      <Row>
        {/* `disabled` drops it from the Tab order and suppresses onClick; the
            dimming is ours, since the component applies no style. */}
        <Button
          disabled
          onClick={() => setCount((c) => c + 1)}
          style={{
            display: "flex",
            paddingX: 16,
            paddingY: 9,
            borderRadius: 8,
            alignItems: "center",
            justifyContent: "center",
            backgroundColor: theme.accent,
            opacity: 0.45,
          }}
        >
          <Text style={{ color: theme.accentText, fontSize: 14, fontWeight: "bold" }}>
            Disabled
          </Text>
        </Button>
        <Label>Disabled</Label>
      </Row>
    </Section>
  );
}
