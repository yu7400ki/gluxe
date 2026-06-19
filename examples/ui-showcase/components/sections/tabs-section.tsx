import { Text, View } from "@gluxe/react";
import { Tabs } from "@gluxe/ui";
import React from "react";

import { theme } from "../theme";
import { Section } from "../ui-kit";

const TABS = [
  {
    value: "account",
    label: "Account",
    heading: "Account Settings",
    body: "Update your display name, email address, and profile picture. Changes to your email require re-verification before they take effect.",
  },
  {
    value: "notifications",
    label: "Notifications",
    heading: "Notification Preferences",
    body: "Choose which events trigger desktop alerts and how long banners stay on screen. Activity digests can be sent daily or weekly.",
  },
  {
    value: "security",
    label: "Security",
    heading: "Security & Privacy",
    body: "Manage your password, active sessions, and two-factor authentication. Revoke any session you no longer recognise to keep your account secure.",
  },
] as const;

export function TabsSection(): React.ReactElement {
  return (
    <Section
      title="Tabs"
      description="A headless tab panel. Selection is triggered on click; keyboard arrow-key navigation is not available in this framework."
    >
      <Tabs defaultValue="account">
        <Tabs.List
          style={{
            display: "flex",
            flexDirection: "row",
            gap: 4,
            padding: 4,
            borderRadius: 10,
            backgroundColor: theme.surfaceHigh,
            alignSelf: "flex-start",
          }}
        >
          {TABS.map((tab) => (
            <Tabs.Trigger key={tab.value} value={tab.value}>
              {({ selected }) => (
                <View
                  style={{
                    paddingLeft: 14,
                    paddingRight: 14,
                    paddingTop: 8,
                    paddingBottom: 8,
                    borderRadius: 8,
                    backgroundColor: selected ? theme.accent : "transparent",
                    transition: { property: "all", duration: 160, easing: "ease-out" },
                    _hover: {
                      backgroundColor: selected ? theme.accent : theme.border,
                    },
                  }}
                >
                  <Text
                    style={{
                      color: selected ? theme.accentText : theme.textMuted,
                      fontWeight: 500,
                      fontSize: 14,
                      _hover: {
                        color: selected ? theme.accentText : theme.text,
                      },
                    }}
                  >
                    {tab.label}
                  </Text>
                </View>
              )}
            </Tabs.Trigger>
          ))}
        </Tabs.List>

        {TABS.map((tab) => (
          <Tabs.Content key={tab.value} value={tab.value}>
            <View
              style={{
                display: "flex",
                marginTop: 10,
                padding: 14,
                borderRadius: 10,
                backgroundColor: theme.surface,
                borderWidth: 1,
                borderColor: theme.border,
                flexDirection: "column",
                gap: 8,
              }}
            >
              <Text style={{ color: theme.text, fontWeight: "bold", fontSize: 15 }}>
                {tab.heading}
              </Text>
              <Text style={{ color: theme.textMuted, fontSize: 13, lineHeight: 1.5 }}>
                {tab.body}
              </Text>
            </View>
          </Tabs.Content>
        ))}
      </Tabs>
    </Section>
  );
}
