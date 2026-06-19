import React, { useEffect, useMemo, useRef, useSyncExternalStore } from "react";

import { LocationContext, RouteContext, RouterContext } from "./context";
import { createMemoryHistory, type MemoryHistory } from "./history";
import { matchRoutes } from "./matcher";
import type { NavigateFunction, RouteObject } from "./types";

export interface RouterProps {
  /** Route tree (hand-written or from `virtual:@gluxe/router/routes`). */
  routes: RouteObject[];
  /** Initial history stack; the last entry is the current location. Default: `["/"]`. */
  initialEntries?: string[];
}

/**
 * Top-level router. Owns the in-memory history, matches the current pathname
 * against `routes`, and renders the matched chain (layouts outside, leaf inside,
 * connected via `<Outlet>`).
 */
export function Router({ routes, initialEntries }: RouterProps): React.ReactElement {
  const historyRef = useRef<MemoryHistory | null>(null);
  if (historyRef.current === null) {
    historyRef.current = createMemoryHistory(initialEntries);
  }
  const history = historyRef.current;

  // If useSyncExternalStore misbehaves under Boa, swap for useState + useEffect.
  // The server-snapshot arg is irrelevant under GPUI but keeps renderToString working.
  const location = useSyncExternalStore(
    history.listen,
    () => history.location,
    () => history.location,
  );

  const routerValue = useMemo(() => {
    const navigate: NavigateFunction = (to, options) => {
      if (typeof to === "number") {
        history.go(to);
      } else if (options?.replace) {
        history.replace(to, options.state);
      } else {
        history.push(to, options?.state);
      }
    };
    return { history, navigate };
  }, [history]);

  const matches = useMemo(
    () => matchRoutes(routes, location.pathname),
    [routes, location.pathname],
  );

  // Warn after commit, not during render: a render-phase side effect can fire
  // multiple times for one pathname (re-renders, StrictMode double-invoke).
  // Keying the effect on the no-match result dedups those automatically.
  useEffect(() => {
    if (matches) return;
    console.warn(
      `[gluxe-router] No route matched "${location.pathname}". ` +
        `Add a "*" route (404.tsx in the file-based router) to render a fallback.`,
    );
  }, [matches, location.pathname]);

  let element: React.ReactElement | null = null;
  if (matches) {
    element = matches.reduceRight<React.ReactElement | null>((outlet, match, index) => {
      const Component = match.route.component;
      return (
        <RouteContext.Provider value={{ matches, index, outlet }}>
          {Component ? <Component /> : outlet}
        </RouteContext.Provider>
      );
    }, null);
  }

  return (
    <RouterContext.Provider value={routerValue}>
      <LocationContext.Provider value={location}>{element}</LocationContext.Provider>
    </RouterContext.Provider>
  );
}
