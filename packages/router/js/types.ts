/**
 * A single node in the route tree consumed by `<Router routes={...} />`.
 * A route with `children` is a layout level whose `component` renders the
 * matched child via `<Outlet />`. A route without a `component` is a
 * pass-through node that contributes its `path` segments but renders nothing.
 */
export interface RouteObject {
  /**
   * Path pattern relative to the parent route, without a leading slash.
   * Segments may be static (`"about"`), dynamic (`":id"`), or `"*"` (wildcard,
   * matches any remaining segments). Multi-segment patterns (`"users/:id"`) are
   * allowed. Omit for index routes and component-less grouping nodes.
   */
  path?: string;
  /** Matches when the URL is exactly the parent path. Mutually exclusive with `path` and `children`. */
  index?: boolean;
  /** Component rendered when this route matches. */
  component?: React.ComponentType;
  /** Nested child routes, rendered through this route's `<Outlet />`. */
  children?: RouteObject[];
  /** Optional identifier; the file-based router fills this in for debugging. */
  id?: string;
}

/** One entry of the in-memory history stack. */
export interface Location {
  /** Absolute pathname, e.g. `"/users/42"`. */
  pathname: string;
  /** Unique key; changes on every navigation. */
  key: string;
  /** Arbitrary state passed via `navigate(to, { state })`. */
  state?: unknown;
}

/** Dynamic path parameters captured during matching. */
export type Params = Record<string, string>;

/** One matched level of the route tree, root first. */
export interface RouteMatch {
  route: RouteObject;
  /** Params accumulated from the root down to this level. */
  params: Params;
  /** Portion of the pathname matched up to this level. */
  pathname: string;
}

export interface NavigateOptions {
  /** Replace the current history entry instead of pushing. */
  replace?: boolean;
  /** State stored on the new location, readable via `useLocation().state`. */
  state?: unknown;
}

/** Navigate to a pathname, or move through history when given a delta number (`-1` = back). */
export type NavigateFunction = (to: string | number, options?: NavigateOptions) => void;
