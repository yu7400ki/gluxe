import { createContext } from "react";

import type { MemoryHistory } from "./history";
import type { Location, NavigateFunction, RouteMatch } from "./types";

/** Navigation surface. Created once per `<Router>` — consumers of `useNavigate()` do not re-render on navigation. */
export interface RouterContextValue {
  history: MemoryHistory;
  navigate: NavigateFunction;
}

export const RouterContext = createContext<RouterContextValue | null>(null);

/** The current location; changes (and re-renders consumers) on navigation. */
export const LocationContext = createContext<Location | null>(null);

/** Per-matched-level context; lets `<Outlet>` and `useParams` find their depth. */
export interface RouteContextValue {
  matches: RouteMatch[];
  /** Index of this level within `matches`. */
  index: number;
  /** Element for the next (deeper) matched level, rendered by `<Outlet>`. */
  outlet: React.ReactElement | null;
}

export const RouteContext = createContext<RouteContextValue | null>(null);
