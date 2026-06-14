import type { Location } from "./types";

/**
 * In-memory history stack (no browser URL bar in gluxe).
 */
export interface MemoryHistory {
  /** Current location. Object identity changes on every navigation. */
  readonly location: Location;
  readonly index: number;
  readonly length: number;
  /** Push a new entry, discarding any forward entries. */
  push(to: string, state?: unknown): void;
  /** Replace the current entry in place. */
  replace(to: string, state?: unknown): void;
  /** Move by `delta` entries, clamped to stack bounds. */
  go(delta: number): void;
  back(): void;
  forward(): void;
  /** Subscribe to navigation. Returns an unsubscribe function. */
  listen(listener: () => void): () => void;
}

function normalizePathname(to: string): string {
  let pathname = to.startsWith("/") ? to : `/${to}`;
  // Collapse a trailing slash (except for the root itself).
  if (pathname.length > 1 && pathname.endsWith("/")) {
    pathname = pathname.slice(0, -1);
  }
  return pathname;
}

export function createMemoryHistory(initialEntries: string[] = ["/"]): MemoryHistory {
  // Incrementing counter — Boa has no crypto global, so no crypto.randomUUID().
  let keyCounter = 0;
  const createLocation = (to: string, state?: unknown): Location => ({
    pathname: normalizePathname(to),
    key: `${keyCounter++}`,
    state,
  });

  let entries: Location[] = (initialEntries.length > 0 ? initialEntries : ["/"]).map((to) =>
    createLocation(to),
  );
  let index = entries.length - 1;
  const listeners = new Set<() => void>();

  const notify = (): void => {
    for (const listener of Array.from(listeners)) listener();
  };

  // Closures (not `this.go(...)`) so destructured methods keep working:
  // `const { back } = history; back();` must not throw.
  const go = (delta: number): void => {
    const next = Math.min(Math.max(index + delta, 0), entries.length - 1);
    if (next === index) return;
    index = next;
    notify();
  };

  return {
    get location() {
      return entries[index];
    },
    get index() {
      return index;
    },
    get length() {
      return entries.length;
    },
    push(to, state) {
      entries = entries.slice(0, index + 1);
      entries.push(createLocation(to, state));
      index = entries.length - 1;
      notify();
    },
    replace(to, state) {
      entries = [...entries];
      entries[index] = createLocation(to, state);
      notify();
    },
    go,
    back: () => go(-1),
    forward: () => go(1),
    listen(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}
