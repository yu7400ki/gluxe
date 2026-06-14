import { useContext, useMemo } from "react";

import { LocationContext, RouteContext, RouterContext } from "./context";
import type { Location, NavigateFunction, Params } from "./types";

function useRouterContext(hookName: string) {
  const ctx = useContext(RouterContext);
  if (!ctx) {
    throw new Error(`${hookName} must be used inside a <Router>`);
  }
  return ctx;
}

/** Returns a stable navigate function. Accepts a pathname or numeric delta (`-1` = back). */
export function useNavigate(): NavigateFunction {
  return useRouterContext("useNavigate()").navigate;
}

/** Returns the current location; re-renders on every navigation. */
export function useLocation(): Location {
  const location = useContext(LocationContext);
  if (!location) {
    throw new Error("useLocation() must be used inside a <Router>");
  }
  return location;
}

/** Returns dynamic path params for the current route level, merged with all ancestor params. */
export function useParams<T extends Params = Params>(): T {
  const ctx = useContext(RouteContext);
  if (!ctx) {
    throw new Error("useParams() must be used inside a route rendered by <Router>");
  }
  return ctx.matches[ctx.index].params as T;
}

export interface HistoryControls {
  back: () => void;
  forward: () => void;
  go: (delta: number) => void;
  index: number;
  length: number;
}

/**
 * Returns history controls plus the current stack position. Subscribes to
 * location changes so `index`/`length` stay current (useful for disabling
 * back/forward buttons).
 */
export function useHistory(): HistoryControls {
  const { history } = useRouterContext("useHistory()");
  const location = useLocation(); // change signal; history object is stable
  return useMemo(
    () => ({
      back: () => history.back(),
      forward: () => history.forward(),
      go: (delta: number) => history.go(delta),
      index: history.index,
      length: history.length,
    }),
    [history, location],
  );
}
