// Floating-overlay positioning props (anchored overlays).

import type { Px, Rem } from "./style-types";

/** Side of the anchor the floating element is placed on. */
export type FloatingSide = "top" | "bottom" | "left" | "right";
/** Alignment of the floating element along the anchor's cross axis. */
export type FloatingAlign = "start" | "center" | "end";
/** Placement area: a side, optionally followed by an alignment (e.g. `"bottom start"`). */
export type FloatingArea = FloatingSide | `${FloatingSide} ${FloatingAlign}`;

/**
 * Positions an element as a floating overlay anchored to a named element
 * (one carrying a matching `anchorName`).
 *
 * The overlay is lifted above in-flow content and clipping, sized automatically,
 * and snapped inside the window on overflow — so it can be authored anywhere in
 * the tree, not only as a child of the anchor.
 */
export interface FloatingProps {
  /** The `anchorName` of the element to anchor to. */
  anchor: string;
  /** Placement relative to the anchor. Default `"bottom start"`. */
  area?: FloatingArea;
  /** Gap from the anchor along the `area` side. Bare number = px; strings accept
   *  `"px"`/`"rem"` (`%`/`auto` are ignored). Default `0`. */
  offset?: number | Px | Rem;
  /** Minimum gap kept from the window edge when the overlay is snapped back
   *  on-screen. Bare number = px; `"px"`/`"rem"` strings accepted. Default `0`. */
  margin?: number | Px | Rem;
  /** Draw order among floating overlays — higher is closer to the viewer. Always
   *  above in-flow content; this is not a general CSS `z-index`. */
  priority?: number;
}
