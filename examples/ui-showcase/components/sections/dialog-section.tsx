// Dialog section — demonstrates the headless Dialog API from @gluxe/ui. All
// visual styling lives here; Dialog supplies only open/close behaviour, focus
// handling, and the portaled overlay + panel through render-function children.

import { Text, View } from "@gluxe/react";
import { Dialog } from "@gluxe/ui";
import React from "react";

import { focusRing, theme } from "../theme";
import { Section } from "../ui-kit";

export function DialogSection(): React.ReactElement {
  return (
    <Section
      title="Dialog"
      description="A modal dialog. The trigger opens it; the panel and a dimming backdrop mount only while open and render through a portal, lifting above the page. Opening focuses the panel; Escape or a click on the backdrop closes it and returns focus to the trigger."
    >
      <Dialog>
        <Dialog.Trigger
          style={{
            alignSelf: "flex-start",
            paddingX: 16,
            paddingY: 9,
            borderRadius: 10,
            backgroundColor: theme.accent,
            transition: { property: "all", duration: 140, easing: "ease-out" },
            _hover: { backgroundColor: theme.accentBright },
            _active: { backgroundColor: theme.accentDim },
            ...focusRing(10),
          }}
        >
          <Text style={{ color: theme.accentText, fontSize: 14, fontWeight: "bold" }}>
            Edit profile
          </Text>
        </Dialog.Trigger>

        <Dialog.Overlay style={{ backgroundColor: "rgba(0, 0, 0, 0.55)" }} />

        <Dialog.Positioner>
          <Dialog.Content
            style={{
              width: 380,
              display: "flex",
              flexDirection: "column",
              gap: 14,
              padding: 22,
              borderRadius: 16,
              borderWidth: 1,
              borderColor: theme.border,
              backgroundColor: theme.surface,
              boxShadow: "2xl",
            }}
          >
            <View style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              <Text style={{ color: theme.text, fontSize: 18, fontWeight: "bold" }}>
                Edit profile
              </Text>
              <Text style={{ color: theme.textMuted, fontSize: 13, lineHeight: 1.5 }}>
                Make changes to your profile here. This panel is rendered only while the dialog is
                open; closing it unmounts the panel and the backdrop entirely.
              </Text>
            </View>

            <View
              style={{
                display: "flex",
                flexDirection: "row",
                justifyContent: "flex-end",
                gap: 10,
                marginTop: 4,
              }}
            >
              <Dialog.Close
                style={{
                  paddingX: 14,
                  paddingY: 8,
                  borderRadius: 9,
                  borderWidth: 1,
                  borderColor: theme.borderHigh,
                  backgroundColor: theme.surfaceHigh,
                  transition: { property: "all", duration: 140, easing: "ease-out" },
                  _hover: { backgroundColor: theme.borderHigh },
                  ...focusRing(9),
                }}
              >
                <Text style={{ color: theme.text, fontSize: 13 }}>Cancel</Text>
              </Dialog.Close>
              <Dialog.Close
                style={{
                  paddingX: 14,
                  paddingY: 8,
                  borderRadius: 9,
                  backgroundColor: theme.accent,
                  transition: { property: "all", duration: 140, easing: "ease-out" },
                  _hover: { backgroundColor: theme.accentBright },
                  _active: { backgroundColor: theme.accentDim },
                  ...focusRing(9),
                }}
              >
                <Text style={{ color: theme.accentText, fontSize: 13, fontWeight: "bold" }}>
                  Save changes
                </Text>
              </Dialog.Close>
            </View>
          </Dialog.Content>
        </Dialog.Positioner>
      </Dialog>
    </Section>
  );
}
