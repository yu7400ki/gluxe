import { Router } from "@gluxe/router";
import { registerRootComponent } from "gluxe";
import { routes } from "virtual:@gluxe/router/routes";

function App() {
  return <Router routes={routes} />;
}

registerRootComponent(App);
