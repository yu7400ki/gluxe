import type { ReactNode } from "react";

/**
 * Children that are either static nodes or a render function receiving the
 * part's current state — the headless way to style by state without CSS
 * attribute selectors (which GPUI does not have).
 */
export type Slot<State> = ReactNode | ((state: State) => ReactNode);

/** Resolve a {@link Slot}: call it with `state` when it is a render function. */
export function renderSlot<State>(children: Slot<State>, state: State): ReactNode {
  return typeof children === "function" ? children(state) : children;
}
