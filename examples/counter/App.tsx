import { View, Text } from "@gluxe/react";
import { setWindowTitle } from "@gluxe/react/window";
import React, { useEffect, useState } from "react";

export default function Counter() {
  const [count, setCount] = useState(0);

  useEffect(() => {
    setWindowTitle(`Counter — ${count}`);
  }, [count]);

  return (
    <View
      style={{
        display: "flex",
        flex: 1,
        flexDirection: "column",
        gap: 12,
        padding: 16,
        alignItems: "center",
      }}
    >
      <View
        style={{
          backgroundColor: "#3d5a80",
          padding: 12,
          borderRadius: 8,
        }}
      >
        <Text style={{ color: "#ffffff", fontSize: 24 }}>Count: {count}</Text>
      </View>

      <View style={{ display: "flex", flexDirection: "row", gap: 8 }}>
        <View
          style={{
            backgroundColor: "#98c1d9",
            padding: 8,
            borderRadius: 4,
            _hover: { backgroundColor: "#5fa8d3" },
            _active: { backgroundColor: "#3d8ab8" },
          }}
          onClick={() => setCount((c) => c - 1)}
        >
          <Text style={{ color: "#1d3557" }}>−1</Text>
        </View>
        <View
          style={{
            backgroundColor: "#e0fbfc",
            padding: 8,
            borderRadius: 4,
            _hover: { backgroundColor: "#b8f0f5" },
            _active: { backgroundColor: "#82d8e0" },
          }}
          onClick={() => setCount(0)}
        >
          <Text style={{ color: "#3d5a80" }}>Reset</Text>
        </View>
        <View
          style={{
            backgroundColor: "#98c1d9",
            padding: 8,
            borderRadius: 4,
            _hover: { backgroundColor: "#5fa8d3" },
            _active: { backgroundColor: "#3d8ab8" },
          }}
          onClick={() => setCount((c) => c + 1)}
        >
          <Text style={{ color: "#1d3557" }}>+1</Text>
        </View>
      </View>
    </View>
  );
}
