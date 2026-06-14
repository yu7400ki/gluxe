import { Text, View } from "gluxe";

import { C } from "../theme";

interface ButtonProps {
  label: string;
  onClick: () => void;
  variant?: "primary" | "ghost";
  /** Larger padding/font for hero buttons. */
  large?: boolean;
}

/** Themed pill button with hover/active feedback. */
export function Button({ label, onClick, variant = "primary", large = false }: ButtonProps) {
  const primary = variant === "primary";
  return (
    <View
      style={{
        paddingX: large ? 36 : 22,
        paddingY: large ? 14 : 10,
        borderRadius: 12,
        cursor: "pointer",
        backgroundColor: primary ? C.gold : "transparent",
        borderWidth: primary ? 0 : 1.5,
        borderColor: C.panelBorder,
        boxShadow: primary
          ? [{ offsetY: 3, blurRadius: 10, spreadRadius: -2, color: "#00000033" }]
          : "none",
        _hover: {
          backgroundColor: primary ? C.goldBright : C.panelHigh,
          borderColor: primary ? C.goldBright : C.hoverBorder,
        },
        _active: {
          backgroundColor: primary ? C.goldDim : C.panel,
        },
      }}
      onClick={onClick}
    >
      <Text
        style={{
          fontSize: large ? 18 : 14,
          fontWeight: "bold",
          color: primary ? "#241c0d" : C.text,
          textAlign: "center",
        }}
      >
        {label}
      </Text>
    </View>
  );
}
