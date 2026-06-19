// ui-showcase — a styled gallery of the headless @gluxe/ui components.
//
// @gluxe/ui ships behaviour and state but zero styles. Every visual in this app
// is supplied here, proving the components are fully style-able. Each section
// lives in components/sections/ and reads component state through
// render-function children to drive its styling.

import { Text, View } from "@gluxe/react";
import type React from "react";

import { AccordionSection } from "./components/sections/accordion-section";
import { ButtonSection } from "./components/sections/button-section";
import { CheckboxSection } from "./components/sections/checkbox-section";
import { DisclosureSection } from "./components/sections/disclosure-section";
import { RadioGroupSection } from "./components/sections/radio-group-section";
import { SwitchSection } from "./components/sections/switch-section";
import { TabsSection } from "./components/sections/tabs-section";
import { ToggleSection } from "./components/sections/toggle-section";
import { theme } from "./components/theme";

export default function App(): React.ReactElement {
  return (
    <View
      style={{
        height: "100%",
        width: "100%",
        overflowY: "scroll",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        backgroundColor: theme.bg,
      }}
    >
      <View
        style={{
          display: "flex",
          flexDirection: "column",
          flexShrink: 0,
          gap: 16,
          padding: 28,
          width: "100%",
          maxWidth: 680,
        }}
      >
        <View style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 4 }}>
          <Text style={{ color: theme.text, fontSize: 26, fontWeight: "bold" }}>Gluxe UI</Text>
          <Text style={{ color: theme.textMuted, fontSize: 14, lineHeight: 1.5 }}>
            A gallery of headless @gluxe/ui components. The library provides behaviour and state
            only — every style on this page is applied by the example. Every control is
            keyboard-navigable: Tab to move between them, the arrow keys to move within a RadioGroup
            or the tab list, and Space or Enter to activate.
          </Text>
        </View>

        <ButtonSection />
        <ToggleSection />
        <CheckboxSection />
        <SwitchSection />
        <RadioGroupSection />
        <DisclosureSection />
        <AccordionSection />
        <TabsSection />
      </View>
    </View>
  );
}
