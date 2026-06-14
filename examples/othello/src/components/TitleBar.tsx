import { useLocation, useNavigate } from "@gluxe/router";
import { Text, View } from "gluxe";

import { C } from "../theme";

interface WindowButtonProps {
  area: "min" | "max" | "close";
  label: string;
  /** Close button gets a red hover tint. */
  danger?: boolean;
}

/**
 * One native window-control region. The OS (or the framework on
 * macOS/Linux) handles the click — no onClick here by design.
 */
function WindowButton({ area, label, danger = false }: WindowButtonProps) {
  return (
    <View
      windowControlArea={area}
      style={{
        width: 46,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        _hover: { backgroundColor: danger ? "#e8112326" : C.panelHigh },
      }}
    >
      <Text style={{ fontSize: 13, color: C.textSecondary }}>{label}</Text>
    </View>
  );
}

/**
 * Custom titlebar for the frameless window: back navigation on the left
 * (game screen only), a drag region with the app title, and window buttons.
 */
export function TitleBar() {
  const location = useLocation();
  const navigate = useNavigate();
  const showBack = location.pathname !== "/";

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "row",
        alignItems: "stretch",
        width: "100%",
        height: 40,
      }}
    >
      {/* Back navigation (kept outside the drag region so onClick works on Windows) */}
      {showBack ? (
        <View
          style={{
            width: 46,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            cursor: "pointer",
            _hover: { backgroundColor: C.panelHigh },
            _active: { backgroundColor: C.panel },
          }}
          onClick={() => navigate("/")}
        >
          <Text style={{ fontSize: 18, fontWeight: "bold", color: C.text }}>←</Text>
        </View>
      ) : (
        <View style={{ width: 16 }} />
      )}

      {/* Drag region (doubles as the title strip; double-click maximizes) */}
      <View
        windowControlArea="drag"
        style={{
          display: "flex",
          flex: 1,
          flexDirection: "row",
          alignItems: "center",
        }}
      >
        <Text style={{ fontSize: 13, fontWeight: "bold", color: C.textSecondary }}>OTHELLO</Text>
      </View>

      {/* Window buttons */}
      <WindowButton area="min" label="–" />
      <WindowButton area="max" label="□" />
      <WindowButton area="close" label="✕" danger />
    </View>
  );
}
