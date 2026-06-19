// GPUI event objects and the handler props host elements accept.

import type { GpuiInstance } from "./instance";

/** Fields shared by all GPUI event objects. */
export interface GpuiEventBase {
  /** ElementId of the element that received the event. */
  target: number;
}

/** Mouse event object passed to mouse-event handlers. */
export interface GpuiMouseEvent extends GpuiEventBase {
  type: "click" | "mousedown" | "mouseup" | "mousemove" | "mouseenter" | "mouseleave";
  /** Logical-pixel position within the window. */
  x: number;
  y: number;
}

/**
 * Keyboard event object passed to `onKeyDown` handlers.
 *
 * `key` carries GPUI's `Keystroke.key` value (e.g. `"down"`, `"up"`,
 * `"enter"`, `"escape"`, `"home"`, `"end"`, `"pageup"`, `"pagedown"`,
 * `"backspace"`, `"a"` … `"z"`).
 */
export interface GpuiKeyboardEvent extends GpuiEventBase {
  type: "keydown";
  key: string;
  shift: boolean;
  ctrl: boolean;
  alt: boolean;
  meta: boolean;
}

/** Focus event passed to `onFocus` / `onBlur`. */
export interface GpuiFocusEvent extends GpuiEventBase {
  type: "focus" | "blur";
  /** Current text value at the time of the event. Present on `<TextInput>` only;
   *  `undefined` for `<View>` / `<Image>` / `<Text>`. */
  value?: string;
}

/** Event handler props supported by host elements (DOM-style naming). */
export interface EventProps {
  onClick?: (e: GpuiMouseEvent) => void;
  onMouseDown?: (e: GpuiMouseEvent) => void;
  onMouseUp?: (e: GpuiMouseEvent) => void;
  onMouseMove?: (e: GpuiMouseEvent) => void;
  onMouseEnter?: (e: GpuiMouseEvent) => void;
  onMouseLeave?: (e: GpuiMouseEvent) => void;
  /** Fires while this element (or a descendant) holds keyboard focus.
   *  Any focus-related prop (below) makes the element focusable. */
  onKeyDown?: (e: GpuiKeyboardEvent) => void;
  /** Fires when this element gains keyboard focus. Implies focusability. */
  onFocus?: (e: GpuiFocusEvent) => void;
  /** Fires when this element loses keyboard focus. Implies focusability. */
  onBlur?: (e: GpuiFocusEvent) => void;
  /**
   * Tab order index (HTML-style). Setting it makes the element focusable:
   * - `>= 0` — reachable via Tab / Shift+Tab, and programmatically focusable.
   * - `-1` — programmatically focusable (`ref.current.focus()`) but skipped by Tab.
   */
  tabIndex?: number;
  /** Override whether the element is a Tab stop. Defaults from `tabIndex`
   *  (`>= 0` → stop). Use `tabStop={false}` on a focusable element to keep it
   *  out of the Tab order while still allowing programmatic focus. */
  tabStop?: boolean;
  /** Ref to this element's {@link GpuiInstance} (`ref.current.focus()` / `.blur()`). */
  ref?: React.Ref<GpuiInstance>;
}

/** Callback for text-input value changes and submit (React Native-style). */
export type TextChangeHandler = (text: string) => void;
