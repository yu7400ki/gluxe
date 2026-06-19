// Disclosure (Collapsible) section — demonstrates the headless Disclosure API
// from @gluxe/ui. All visual styling lives here; the component supplies only
// behaviour and open/closed state through render-function children.

import { Text, View } from "@gluxe/react";
import { Disclosure } from "@gluxe/ui";
import React from "react";

import { theme } from "../theme";
import { Section } from "../ui-kit";

export function DisclosureSection(): React.ReactElement {
  return (
    <Section
      title="Disclosure (Collapsible)"
      description="A headless expand/collapse region. The trigger header and content panel are styled here; Disclosure supplies open state and toggle behaviour with no built-in styles."
    >
      <Disclosure defaultOpen={false} style={{ display: "flex", flexDirection: "column" }}>
        <>
          <Disclosure.Trigger>
            {({ open }) => (
              <View
                style={{
                  display: "flex",
                  flexDirection: "row",
                  justifyContent: "space-between",
                  alignItems: "center",
                  padding: 12,
                  borderRadius: 10,
                  backgroundColor: theme.surfaceHigh,
                  transition: { property: "all", duration: 150, easing: "ease-out" },
                  _hover: { backgroundColor: theme.borderHigh },
                }}
              >
                <Text style={{ color: theme.text, fontSize: 14, fontWeight: "bold" }}>
                  What is a headless component?
                </Text>
                <Text
                  style={{
                    color: open ? theme.accent : theme.textMuted,
                    fontSize: 14,
                    transition: { property: "all", duration: 150, easing: "ease-out" },
                  }}
                >
                  {open ? "▾" : "▸"}
                </Text>
              </View>
            )}
          </Disclosure.Trigger>
          <Disclosure.Content>
            <View
              style={{
                display: "flex",
                marginTop: 8,
                padding: 12,
                borderRadius: 10,
                backgroundColor: theme.surface,
                borderWidth: 1,
                borderColor: theme.border,
                flexDirection: "column",
                gap: 8,
              }}
            >
              <Text style={{ color: theme.textMuted, fontSize: 13, lineHeight: 1.5 }}>
                A headless component provides behaviour and state — click handling, open/closed
                toggling, disabled logic — with absolutely no built-in styles. You own the entire
                visual layer through render-function children.
              </Text>
              <Text style={{ color: theme.textMuted, fontSize: 13, lineHeight: 1.5 }}>
                This panel is rendered only while the disclosure is open. Collapsing it unmounts the
                content entirely, keeping the view tree lean.
              </Text>
            </View>
          </Disclosure.Content>
        </>
      </Disclosure>
    </Section>
  );
}
