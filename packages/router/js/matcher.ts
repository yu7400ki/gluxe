import type { Params, RouteMatch, RouteObject } from "./types";

// Specificity scores per segment type, compared lexicographically across branches.
// Static beats dynamic at the same depth; wildcard loses to everything; index wins all.
const STATIC_SCORE = 3;
const DYNAMIC_SCORE = 2;
const SPLAT_SCORE = 1;
const INDEX_SCORE = 4;

/** A root→leaf chain through the route tree, with its specificity score. */
interface Branch {
  routes: RouteObject[];
  score: number[];
}

function splitPath(path: string): string[] {
  return path.split("/").filter(Boolean);
}

function scoreSegment(segment: string): number {
  if (segment === "*") return SPLAT_SCORE;
  if (segment.startsWith(":")) return DYNAMIC_SCORE;
  return STATIC_SCORE;
}

function validateRoute(route: RouteObject, segments: string[]): void {
  const label = route.id ?? route.path ?? (route.index ? "<index>" : "<pathless>");
  if (route.index && (route.path !== undefined || route.children !== undefined)) {
    throw new Error(
      `[gluxe-router] An index route cannot have "path" or "children" (route "${label}").`,
    );
  }
  const splatAt = segments.indexOf("*");
  if (splatAt !== -1 && splatAt !== segments.length - 1) {
    throw new Error(
      `[gluxe-router] "*" is only allowed as the last segment of a path (route "${label}").`,
    );
  }
}

/**
 * Flatten a route tree into matchable root→leaf branches.
 * Layout routes (`children` present) never terminate a branch — only leaves match.
 */
export function flattenRoutes(routes: RouteObject[]): Branch[] {
  const branches: Branch[] = [];
  const walk = (nodes: RouteObject[], chain: RouteObject[], score: number[]): void => {
    for (const route of nodes) {
      const segments = route.path ? splitPath(route.path) : [];
      validateRoute(route, segments);
      const routeScore = route.index ? [INDEX_SCORE] : segments.map(scoreSegment);
      const nextChain = [...chain, route];
      const nextScore = [...score, ...routeScore];
      if (route.children && route.children.length > 0) {
        walk(route.children, nextChain, nextScore);
      } else {
        branches.push({ routes: nextChain, score: nextScore });
      }
    }
  };
  walk(routes, [], []);
  return branches;
}

// Lexicographic descending; longer score beats its own prefix (more segments = more specific).
// Ties preserve declaration order (stable sort).
function compareScores(a: number[], b: number[]): number {
  const len = Math.min(a.length, b.length);
  for (let i = 0; i < len; i++) {
    if (a[i] !== b[i]) return b[i] - a[i];
  }
  return b.length - a.length;
}

// Keyed by routes array identity — mutating the cached array returns stale results.
const branchCache = new WeakMap<RouteObject[], Branch[]>();

function getRankedBranches(routes: RouteObject[]): Branch[] {
  let branches = branchCache.get(routes);
  if (!branches) {
    branches = flattenRoutes(routes).toSorted((a, b) => compareScores(a.score, b.score));
    branchCache.set(routes, branches);
  }
  return branches;
}

function safeDecode(segment: string): string {
  try {
    return decodeURIComponent(segment);
  } catch {
    return segment;
  }
}

function matchBranch(branch: Branch, segments: string[]): RouteMatch[] | null {
  const matches: RouteMatch[] = [];
  let params: Params = {};
  let consumed = 0;
  let sawSplat = false;

  for (const route of branch.routes) {
    if (route.index) {
      if (consumed !== segments.length) return null;
    } else if (route.path) {
      for (const part of splitPath(route.path)) {
        if (part === "*") {
          // Decode after splitting so an encoded "/" (%2F) can't create extra segments.
          Object.assign(params, { "*": segments.slice(consumed).map(safeDecode).join("/") });
          consumed = segments.length;
          sawSplat = true;
          break;
        }
        if (part.startsWith(":")) {
          if (consumed >= segments.length) return null;
          Object.assign(params, { [part.slice(1)]: safeDecode(segments[consumed]) });
          consumed++;
        } else {
          if (consumed >= segments.length || segments[consumed] !== part) return null;
          consumed++;
        }
      }
    }
    matches.push({
      route,
      params: { ...params },
      pathname: `/${segments.slice(0, consumed).join("/")}`,
    });
  }

  if (consumed !== segments.length && !sawSplat) return null; // unmatched trailing segments
  return matches;
}

/**
 * Match a pathname against a route tree. Returns the chain of matches from
 * the root down to the leaf, or `null` if nothing matches.
 */
export function matchRoutes(routes: RouteObject[], pathname: string): RouteMatch[] | null {
  const segments = pathname.split("/").filter(Boolean);
  for (const branch of getRankedBranches(routes)) {
    const matches = matchBranch(branch, segments);
    if (matches) return matches;
  }
  return null;
}
