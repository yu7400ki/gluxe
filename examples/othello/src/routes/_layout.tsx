import { Outlet } from "@gluxe/router";
import { View } from "gluxe";

import { TitleBar } from "../components/TitleBar";
import { C } from "../theme";

/** App shell: custom titlebar + flat warm light background behind every screen. */
export default function Layout() {
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        backgroundColor: C.bg,
      }}
    >
      <TitleBar />
      <View style={{ display: "flex", flexDirection: "column", flex: 1 }}>
        <Outlet />
      </View>
    </View>
  );
}
