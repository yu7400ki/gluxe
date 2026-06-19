// ui-showcase — a styled gallery of the headless @gluxe/ui components.
//
// @gluxe/ui ships behaviour and state but zero styles. Every visual in this app
// is supplied here, proving the components are fully style-able. Each section
// lives in components/sections/ and reads component state through
// render-function children to drive its styling.

import { Text, View } from "@gluxe/react";
import { ScrollArea } from "@gluxe/ui";
import type React from "react";

import { AccordionSection } from "./components/sections/accordion-section";
import { ButtonSection } from "./components/sections/button-section";
import { CheckboxSection } from "./components/sections/checkbox-section";
import { DisclosureSection } from "./components/sections/disclosure-section";
import { RadioGroupSection } from "./components/sections/radio-group-section";
import { ScrollAreaSection } from "./components/sections/scroll-area-section";
import { SelectSection } from "./components/sections/select-section";
import { SwitchSection } from "./components/sections/switch-section";
import { TabsSection } from "./components/sections/tabs-section";
import { ToggleSection } from "./components/sections/toggle-section";
import { theme } from "./components/theme";

export default function App(): React.ReactElement {
  return (
    // The whole page scrolls through a ScrollArea, so it gets the same native
    // scrollbar as the components below (the ScrollArea section is then a nested
    // example). The root is `position: relative` so the track overlays the edge.
    <ScrollArea
      style={{
        position: "relative",
        height: "100%",
        width: "100%",
        backgroundColor: theme.bg,
      }}
    >
      <ScrollArea.Viewport
        style={{
          height: "100%",
          width: "100%",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
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
              keyboard-navigable: Tab to move between them, the arrow keys to move within a
              RadioGroup or the tab list, and Space or Enter to activate.
            </Text>
          </View>

          <ButtonSection />
          <ToggleSection />
          <CheckboxSection />
          <SwitchSection />
          <RadioGroupSection />
          <SelectSection />
          <DisclosureSection />
          <AccordionSection />
          <TabsSection />
          <ScrollAreaSection />
        </View>
      </ScrollArea.Viewport>

      <ScrollArea.Scrollbar
        style={{
          position: "absolute",
          top: 4,
          right: 4,
          bottom: 4,
          width: 10,
          borderRadius: 5,
          backgroundColor: theme.track,
        }}
      >
        <ScrollArea.Thumb
          style={{
            backgroundColor: theme.accent,
            borderRadius: 5,
            minHeight: 32,
            margin: 1,
            _hover: { backgroundColor: theme.accentBright },
            _active: { backgroundColor: theme.accentDim },
          }}
        />
      </ScrollArea.Scrollbar>
    </ScrollArea>
  );
}
