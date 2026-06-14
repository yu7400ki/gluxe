import { registerRootComponent } from "@gluxe/react";
import { Router } from "@gluxe/router";
import { routes } from "virtual:@gluxe/router/routes";

function App() {
  return <Router routes={routes} />;
}

registerRootComponent(App);
