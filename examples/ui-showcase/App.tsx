// ui-showcase — a styled gallery of the headless @gluxe/ui components.
//
// @gluxe/ui ships behaviour and state but zero styles. Every visual in this app
// is supplied here, proving the components are fully style-able. Each section
// lives in components/sections/ and reads component state through
// render-function children to drive its styling.

import { Text, TextInput, View } from "@gluxe/react";
import { ScrollArea } from "@gluxe/ui";
import React, { useState } from "react";

import { AccordionSection } from "./components/sections/accordion-section";
import { ButtonSection } from "./components/sections/button-section";
import { CheckboxSection } from "./components/sections/checkbox-section";
import { DialogSection } from "./components/sections/dialog-section";
import { DisclosureSection } from "./components/sections/disclosure-section";
import { RadioGroupSection } from "./components/sections/radio-group-section";
import { ScrollAreaSection } from "./components/sections/scroll-area-section";
import { SelectSection } from "./components/sections/select-section";
import { SwitchSection } from "./components/sections/switch-section";
import { TabsSection } from "./components/sections/tabs-section";
import { ToggleSection } from "./components/sections/toggle-section";
import { theme } from "./components/theme";
import { Label, Section } from "./components/ui-kit";

// A multi-line TextInput demo. TextInput is a @gluxe/react primitive (not a
// headless @gluxe/ui part), but it follows the same "the example supplies every
// style" rule, so it earns a card alongside the rest of the gallery. The shared
// field styling lives in one place so the three variants only differ by their
// auto-grow bounds.
const fieldStyle = {
  width: "100%" as const,
  paddingX: 12,
  paddingY: 9,
  borderRadius: 10,
  borderWidth: 1,
  borderColor: theme.borderHigh,
  backgroundColor: theme.surfaceHigh,
  color: theme.text,
  fontSize: 14,
  lineHeight: 1.5,
  caretColor: theme.accent,
  selectionColor: "rgba(110, 168, 254, 0.3)",
  placeholderColor: theme.textMuted,
  _focusVisible: { borderColor: theme.accent },
};

function TextInputSection(): React.ReactElement {
  const [note, setNote] = useState("");
  const [bio, setBio] = useState("");
  const [log, setLog] = useState("");
  const [submitted, setSubmitted] = useState<string | null>(null);

  return (
    <Section
      title="TextInput (multi-line)"
      description="A multi-line text field. Enter inserts a newline and the box grows with its content, soft-wrapping at its width; minRows sets a taller floor and maxRows caps the height so the rest scrolls internally. Press Cmd/Ctrl+Enter to submit. Every input here is controlled via useState + onChangeText."
    >
      <View style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <Label>Auto-growing — starts at one row, grows as you add lines</Label>
        <TextInput
          multiline
          value={note}
          onChangeText={setNote}
          onSubmit={setSubmitted}
          placeholder="Write a note… (Cmd/Ctrl+Enter to submit)"
          style={fieldStyle}
        />
      </View>

      <View style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <Label>minRows={3} — opens taller, still grows past three lines</Label>
        <TextInput
          multiline
          minRows={3}
          value={bio}
          onChangeText={setBio}
          onSubmit={setSubmitted}
          placeholder="A short bio…"
          style={fieldStyle}
        />
      </View>

      <View style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <Label>maxRows={5} — grows to five rows, then scrolls inside</Label>
        <TextInput
          multiline
          maxRows={5}
          value={log}
          onChangeText={setLog}
          onSubmit={setSubmitted}
          placeholder="Paste a long log and watch it cap at five rows…"
          style={fieldStyle}
        />
      </View>

      <Text style={{ color: theme.textMuted, fontSize: 13, lineHeight: 1.5 }}>
        {submitted === null
          ? "Nothing submitted yet — press Cmd/Ctrl+Enter in any field above."
          : submitted.trim() === ""
            ? "Submitted an empty value."
            : `Submitted: ${submitted}`}
      </Text>
    </Section>
  );
}

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
          display: "grid",
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
            marginX: "auto",
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
          <TextInputSection />
          <DialogSection />
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
