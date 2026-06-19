// Native host components: low-level building blocks that map to GPUI components
// registered in Rust under reserved type names. Kept off the main `@gluxe/react`
// entry and exposed via the `@gluxe/react/components` subpath; consumed by
// higher-level libraries such as `@gluxe/ui`. Add future native components here.

import { createElement } from "react";

import type { StyleProps } from "./primitives";

// --- Scrollbar (`__GluxeScrollbar`) ----------------------------------------
// Paints a draggable scrollbar thumb inside a consumer-styled track, tracking
// the scroll state of a target viewport element. Backs `@gluxe/ui`'s ScrollArea.

/** Host element type string registered in Rust for the native scrollbar. */
export const ScrollbarHost = "__GluxeScrollbar" as const;

/** Props read by the native `__GluxeScrollbar` host element. */
export interface ScrollbarHostProps {
  /**
   * ElementId of the scrollable viewport whose scroll state the thumb tracks
   * (read from the viewport host element's `ref` — `GpuiInstance.id`). When
   * omitted/`undefined` the native element renders nothing.
   */
  target?: number;
  /** Axis the scrollbar represents. @default "vertical" */
  orientation?: "vertical" | "horizontal";
  /** Minimum thumb length along the main axis, in px. @default 20 */
  minThumbLength?: number;
  /** Thumb fill colour. Accepts any CSS colour string (same formats as
   *  `style.backgroundColor`, e.g. `"#3d5a80"`, `"rgba(...)"`). */
  thumbColor?: string;
  /** Thumb fill colour while the pointer is over the thumb. Falls back to
   *  `thumbColor` when unset. */
  thumbHoverColor?: string;
  /** Thumb fill colour while the thumb is being dragged. Falls back to
   *  `thumbHoverColor` then `thumbColor` when unset. */
  thumbActiveColor?: string;
  /** Thumb corner radius, in px. */
  thumbRadius?: number;
  /** Gap between the thumb and the track's side walls, in px. Inset on the
   *  **cross axis only** (the thumb's length is derived from scroll position and
   *  is never inset). */
  thumbInset?: number;
  /** Styles the scrollbar **track** (the div the native element paints the thumb
   *  inside). Position it yourself — e.g. `position: "absolute"` along an edge. */
  style?: StyleProps;
}

/**
 * Typed JSX wrapper for the native `__GluxeScrollbar` host element. The
 * reconciler passes every non-event prop through to Rust as raw JSON, and
 * `style` styles the track. Consumed by `@gluxe/ui`'s `ScrollArea`; not part of
 * the public `@gluxe/react` entry.
 */
export function ScrollbarHostElement(props: ScrollbarHostProps): React.ReactElement {
  return createElement(ScrollbarHost, props);
}
ScrollbarHostElement.displayName = ScrollbarHost;
