// Small presentational helpers shared across the showcase sections. These are
// ordinary styled `<View>`/`<Text>` wrappers — nothing from @gluxe/ui — used to
// give every section a consistent card layout.

import { Text, View } from "@gluxe/react";
import type React from "react";

import { theme } from "./theme";

export interface SectionProps {
  title: string;
  description: string;
  children: React.ReactNode;
}

/** A titled card that frames one component demo. */
export function Section({ title, description, children }: SectionProps): React.ReactElement {
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 12,
        padding: 20,
        borderRadius: 14,
        backgroundColor: theme.surface,
        borderWidth: 1,
        borderColor: theme.border,
      }}
    >
      <View style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        <Text style={{ color: theme.text, fontSize: 16, fontWeight: "bold" }}>{title}</Text>
        <Text style={{ color: theme.textMuted, fontSize: 13, lineHeight: 1.4 }}>{description}</Text>
      </View>
      <View style={{ display: "flex", flexDirection: "column", gap: 10 }}>{children}</View>
    </View>
  );
}

/** A horizontal row used to place a control next to its label. */
export function Row({ children }: { children: React.ReactNode }): React.ReactElement {
  return (
    <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 12 }}>
      {children}
    </View>
  );
}

/** Inline label text used next to controls. */
export function Label({ children }: { children: React.ReactNode }): React.ReactElement {
  return <Text style={{ color: theme.text, fontSize: 14 }}>{children}</Text>;
}
