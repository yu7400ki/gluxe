// Accordion demo section — an FAQ accordion built on the headless
// @gluxe/ui `Accordion`. All visual styling is applied here; the
// headless component supplies only open/close behaviour and state.

import { Text, View } from "@gluxe/react";
import { Accordion } from "@gluxe/ui";
import React from "react";

import { focusRing, theme } from "../theme";
import { Section } from "../ui-kit";

const faqs = [
  {
    value: "q1",
    q: "Why does Accordion ship with no styles?",
    a: "Gluxe UI components are headless by design — they own behaviour and state only. You supply every visual detail in JSX, so the component fits any design system without overrides or specificity fights.",
  },
  {
    value: "q2",
    q: "How do I style a trigger differently when its item is open?",
    a: "Accordion.Trigger passes { open, disabled, value } to its render-function children. Read open to swap colours, rotate an icon, or change any style prop — no extra state required.",
  },
  {
    value: "q3",
    q: "Does the Accordion support keyboard navigation between items?",
    a: "Yes. Each trigger is reachable with Tab and expands or collapses on Space or Enter, with a focus ring shown while you navigate with the keyboard. Disabled triggers are skipped in the Tab order.",
  },
];

export function AccordionSection(): React.ReactElement {
  return (
    <Section
      title="Accordion"
      description="A headless expandable list. Open/close state is managed by the component; every colour and layout detail is defined here. Each trigger is a Tab stop that expands or collapses on Space or Enter."
    >
      <Accordion
        type="single"
        collapsible
        defaultValue={faqs[0].value}
        style={{ display: "flex", flexDirection: "column", gap: 8 }}
      >
        {faqs.map((item) => (
          <Accordion.Item
            key={item.value}
            value={item.value}
            style={{ display: "flex", flexDirection: "column" }}
          >
            {/* Accordion.Trigger is the focusable node — Tab to it, Space / Enter
                to expand. Ring radius matches the header's top corners. */}
            <Accordion.Trigger
              style={{ display: "flex", flexDirection: "column", ...focusRing(10) }}
            >
              {({ open }) => (
                <View
                  style={{
                    display: "flex",
                    flexDirection: "row",
                    justifyContent: "space-between",
                    alignItems: "center",
                    padding: 12,
                    backgroundColor: theme.surfaceHigh,
                    borderTopLeftRadius: 10,
                    borderTopRightRadius: 10,
                    borderBottomLeftRadius: open ? 0 : 10,
                    borderBottomRightRadius: open ? 0 : 10,
                    transition: { property: "backgroundColor", duration: 160, easing: "ease-out" },
                    _hover: { backgroundColor: theme.borderHigh },
                  }}
                >
                  <Text style={{ color: theme.text, fontSize: 14 }}>{item.q}</Text>
                  <Text
                    style={{
                      color: open ? theme.accent : theme.textMuted,
                      fontSize: 18,
                      fontWeight: "bold",
                    }}
                  >
                    {open ? "−" : "+"}
                  </Text>
                </View>
              )}
            </Accordion.Trigger>
            <Accordion.Content>
              <View
                style={{
                  display: "flex",
                  flexDirection: "column",
                  padding: 12,
                  backgroundColor: theme.surfaceHigh,
                  borderBottomLeftRadius: 10,
                  borderBottomRightRadius: 10,
                  borderTopWidth: 1,
                  borderColor: theme.border,
                }}
              >
                <Text style={{ color: theme.textMuted, fontSize: 13, lineHeight: 1.5 }}>
                  {item.a}
                </Text>
              </View>
            </Accordion.Content>
          </Accordion.Item>
        ))}
      </Accordion>
    </Section>
  );
}
