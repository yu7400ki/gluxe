import { Text, View } from "@gluxe/react";

import { C } from "../theme";

interface ToastProps {
  message: string;
}

/** A centred translucent toast overlaid on the board (e.g. pass notice). */
export function Toast({ message }: ToastProps) {
  return (
    <View
      style={{
        position: "absolute",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <View
        style={{
          paddingX: 28,
          paddingY: 14,
          borderRadius: 12,
          backgroundColor: "#fffdf6f2",
          borderWidth: 1,
          borderColor: C.selBorder,
          boxShadow: [{ offsetY: 4, blurRadius: 18, spreadRadius: -4, color: "#00000055" }],
        }}
      >
        <Text style={{ fontSize: 20, fontWeight: "bold", color: C.text, textAlign: "center" }}>
          {message}
        </Text>
      </View>
    </View>
  );
}
