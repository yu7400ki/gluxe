import { type GpuiInstance, type StyleProps, View, type ViewProps } from "@gluxe/react";
import { ScrollbarHostElement, type ScrollbarHostProps } from "@gluxe/react/components";
import React, { useCallback, useMemo, useState } from "react";

import { createSafeContext } from "../internal/context";
import { mergeRefs } from "../internal/merge-refs";

/** Axis a `ScrollArea.Scrollbar` (and the native thumb) represents. */
export type ScrollAreaOrientation = "vertical" | "horizontal";

interface ScrollAreaContextValue {
  /** ElementId of the mounted `ScrollArea.Viewport`, or `undefined` before its
   *  ref commits (or after it unmounts). */
  viewportId: number | undefined;
  setViewportId: (id: number | undefined) => void;
}

const [ScrollAreaContextProvider, useScrollAreaContext] =
  createSafeContext<ScrollAreaContextValue>("ScrollArea");

export interface ScrollAreaProps extends Omit<ViewProps, "children"> {
  /** Compound-component children: `ScrollArea.Viewport` and
   *  `ScrollArea.Scrollbar`. */
  children?: React.ReactNode;
}

/**
 * A scrollable region with native scrollbar thumbs. Wraps a
 * {@link ScrollAreaViewport} (the clipped, scrollable content) and one or more
 * {@link ScrollAreaScrollbar}s whose thumbs track the viewport's scroll state.
 *
 * Headless: no styles are applied. Give the root a size and
 * `position: "relative"` so an absolutely-positioned scrollbar track can be
 * placed along an edge.
 */
export function ScrollArea({ children, ...viewProps }: ScrollAreaProps): React.ReactElement {
  const [viewportId, setViewportId] = useState<number | undefined>(undefined);

  const ctx = useMemo<ScrollAreaContextValue>(() => ({ viewportId, setViewportId }), [viewportId]);

  return (
    <ScrollAreaContextProvider value={ctx}>
      {/* Zero styles: the consumer is expected to set `position: "relative"`
          (plus a size) so the scrollbar track can absolutely position along an edge. */}
      <View {...viewProps}>{children}</View>
    </ScrollAreaContextProvider>
  );
}
ScrollArea.displayName = "ScrollArea";

export interface ScrollAreaViewportProps extends ViewProps {}

/**
 * The clipped, scrollable content region. Defaults to `overflowY: "scroll"` so
 * the runtime tracks vertical scroll; override via `style` for other axes (e.g.
 * a horizontal-only area sets `overflowX: "scroll", overflowY: "hidden"`).
 *
 * Registers its host id with the {@link ScrollArea} so sibling scrollbars can
 * track it; clears it on unmount.
 */
export function ScrollAreaViewport({
  style,
  ref,
  ...viewProps
}: ScrollAreaViewportProps): React.ReactElement {
  const ctx = useScrollAreaContext();

  // Stable ref callback: publish the host id on mount, clear it on unmount.
  // `instance` is the GpuiInstance (`{ id, focus, blur }`); guard for a numeric
  // `id` so a non-GpuiInstance ref (e.g. a DOM node under test) is ignored.
  const setViewportRef = useCallback(
    (instance: GpuiInstance | null) => {
      if (instance && typeof instance.id === "number") {
        ctx.setViewportId(instance.id);
      } else {
        ctx.setViewportId(undefined);
      }
    },
    [ctx.setViewportId],
  );

  return (
    <View
      {...viewProps}
      ref={mergeRefs(ref, setViewportRef)}
      style={{ overflowY: "scroll", ...style }}
    />
  );
}
ScrollAreaViewport.displayName = "ScrollArea.Viewport";

export interface ScrollAreaScrollbarProps {
  /** Axis this scrollbar represents. @default "vertical" */
  orientation?: ScrollAreaOrientation;
  /** The single `ScrollArea.Thumb` whose `style` configures the native thumb. */
  children?: React.ReactNode;
  /** Styles the scrollbar **track**. Position it yourself — e.g.
   *  `position: "absolute"` along an edge of the (relative) root. */
  style?: StyleProps;
}

/**
 * A scrollbar track with a native draggable thumb tracking the
 * {@link ScrollAreaViewport}'s scroll state. Place a single
 * {@link ScrollAreaThumb} child to configure the thumb's appearance.
 *
 * Renders `null` until the viewport's ref has committed (so nothing shows on the
 * first render). Style the track via `style`; style the thumb via the
 * `ScrollArea.Thumb` child's `style`.
 */
export function ScrollAreaScrollbar({
  orientation = "vertical",
  children,
  style,
}: ScrollAreaScrollbarProps): React.ReactElement | null {
  const ctx = useScrollAreaContext();

  // Pull the (single) Thumb child's style and map it onto the native thumb props.
  const thumbStyle = extractThumbStyle(children);
  const thumbProps = mapThumbStyle(thumbStyle, orientation);

  // Nothing to track yet — wait for the viewport ref to commit.
  if (ctx.viewportId === undefined) {
    return null;
  }

  return (
    <ScrollbarHostElement
      target={ctx.viewportId}
      orientation={orientation}
      style={style}
      {...thumbProps}
    />
  );
}
ScrollAreaScrollbar.displayName = "ScrollArea.Scrollbar";

/**
 * The subset of style fields the native thumb actually honours. The thumb is
 * painted natively (not a real element), so only these map onto it — anything
 * else has no effect, which is why this is a narrowed type rather than full
 * `StyleProps`.
 */
export interface ScrollAreaThumbStyle {
  /** Thumb fill colour. */
  backgroundColor?: string;
  /** Thumb corner radius, in px. */
  borderRadius?: number;
  /** Minimum thumb length along the scroll axis, in px (vertical scrollbar). */
  minHeight?: number;
  /** Minimum thumb length along the scroll axis, in px (horizontal scrollbar). */
  minWidth?: number;
  /**
   * Gap between the thumb and the track's side walls, in px. Unlike CSS
   * `margin`, only the **cross axis** is inset: for a vertical scrollbar this
   * narrows the thumb's width (a left/right gap); the thumb's length is derived
   * from the scroll position and is never inset, so the main-axis margin has no
   * effect.
   */
  margin?: number;
  /** Fill colour while the pointer is over the thumb. */
  _hover?: { backgroundColor?: string };
  /** Fill colour while the thumb is being dragged. */
  _active?: { backgroundColor?: string };
}

export interface ScrollAreaThumbProps {
  /** Styles the native thumb. Only the fields on {@link ScrollAreaThumbStyle}
   *  are honoured (the thumb is painted natively, not rendered as an element). */
  style?: ScrollAreaThumbStyle;
}

/**
 * Marker for the native scrollbar thumb. Renders nothing of its own — the
 * native scrollbar element paints the thumb. It exists so a scrollbar reads as a
 * normal compound: `<ScrollArea.Scrollbar><ScrollArea.Thumb style={…}/></…>`.
 * Its `style` is read by {@link ScrollAreaScrollbar} (see {@link ScrollAreaThumbProps}).
 */
export function ScrollAreaThumb(_props: ScrollAreaThumbProps): React.ReactElement | null {
  return null;
}
ScrollAreaThumb.displayName = "ScrollArea.Thumb";

/** Find the single `ScrollArea.Thumb` child and return its `style` (if any).
 *  Matches on the component reference, not displayName, for reliability. */
function extractThumbStyle(children: React.ReactNode): ScrollAreaThumbStyle | undefined {
  let thumbStyle: ScrollAreaThumbStyle | undefined;
  React.Children.forEach(children, (child) => {
    if (React.isValidElement(child) && child.type === ScrollAreaThumb) {
      thumbStyle = (child.props as ScrollAreaThumbProps).style;
    }
  });
  return thumbStyle;
}

type MappedThumbProps = Pick<
  ScrollbarHostProps,
  | "thumbColor"
  | "thumbHoverColor"
  | "thumbActiveColor"
  | "thumbRadius"
  | "minThumbLength"
  | "thumbInset"
>;

/** Map a Thumb `style` onto the native `__GluxeScrollbar` thumb props. Each
 *  field is only forwarded when present, so native defaults apply otherwise.
 *  `_hover` / `_active` background colours become the native hover/active fills. */
function mapThumbStyle(
  style: ScrollAreaThumbStyle | undefined,
  orientation: ScrollAreaOrientation,
): MappedThumbProps {
  if (!style) return {};

  const props: MappedThumbProps = {};

  if (typeof style.backgroundColor === "string") {
    props.thumbColor = style.backgroundColor;
  }
  if (typeof style.borderRadius === "number") {
    props.thumbRadius = style.borderRadius;
  }
  const minLength = orientation === "vertical" ? style.minHeight : style.minWidth;
  if (typeof minLength === "number") {
    props.minThumbLength = minLength;
  }
  if (typeof style.margin === "number") {
    props.thumbInset = style.margin;
  }
  if (typeof style._hover?.backgroundColor === "string") {
    props.thumbHoverColor = style._hover.backgroundColor;
  }
  if (typeof style._active?.backgroundColor === "string") {
    props.thumbActiveColor = style._active.backgroundColor;
  }

  return props;
}

ScrollArea.Viewport = ScrollAreaViewport;
ScrollArea.Scrollbar = ScrollAreaScrollbar;
ScrollArea.Thumb = ScrollAreaThumb;
