// Host element type strings, the host-element prop interfaces, and the JSX
// augmentation. The value types — style props, events, floating, and the ref
// instance — live in sibling modules (style-types / events / floating /
// instance) and are re-exported here so `@gluxe/react` keeps a single import
// surface.

import type { EventProps, GpuiFocusEvent, TextChangeHandler } from "./events";
import type { FloatingProps } from "./floating";
import type { GpuiInstance } from "./instance";
import type { StyleProps } from "./style-types";

export type {
  EventProps,
  GpuiEventBase,
  GpuiFocusEvent,
  GpuiKeyboardEvent,
  GpuiMouseEvent,
  TextChangeHandler,
} from "./events";
export type { FloatingAlign, FloatingArea, FloatingProps, FloatingSide } from "./floating";
export type { GpuiInstance } from "./instance";
export type {
  BaseStyleProps,
  BoxShadowValue,
  Color,
  StyleProps,
  TransitionEasing,
  TransitionValue,
} from "./style-types";

// Host element type strings — routed through the reconciler's hostConfig.
export const View = "View" as const;
export const Text = "Text" as const;
export const Image = "Image" as const;
export const TextInput = "TextInput" as const;

export type ViewType = typeof View;
export type TextType = typeof Text;
export type ImageType = typeof Image;
export type TextInputType = typeof TextInput;
export type HostType = ViewType | TextType | ImageType | TextInputType;

export interface ViewProps extends EventProps {
  style?: StyleProps;
  children?: React.ReactNode;
  /** Focuses this element immediately on mount. Implies focusability (same as `onKeyDown`).
   *  Only one element per screen should use this to avoid ambiguity. */
  autoFocus?: boolean;
  /**
   * Window control region for custom titlebars (`window.titlebar: false` in app.json).
   *
   * - `"drag"` — drag-to-move. Double-click maximises/restores. On Windows, handled
   *   natively via NCHITTEST (includes Snap Layouts).
   * - `"close"` / `"max"` / `"min"` — window buttons. On Windows: OS-native.
   *   On macOS/Linux: framework handlers (`remove_window`, `zoom_window`, `minimize_window`).
   *
   * Place close/max/min **inside** the drag region (`.occlude()` is applied automatically
   * so buttons win the hit-test). Do **not** combine with `onClick` on the same element:
   * on Windows, control regions receive non-client messages, so JS handlers only fire
   * on macOS/Linux.
   */
  windowControlArea?: "drag" | "close" | "max" | "min";
  /** Marks this element as a named anchor that `floating` elements can target. */
  anchorName?: string;
  /**
   * Positions this element as a floating overlay anchored to a named element.
   * The overlay blocks the mouse over its area, so clicks land on it rather than
   * falling through to the content it floats above.
   */
  floating?: FloatingProps;
  /**
   * Whether this element blocks the mouse from reaching elements painted behind
   * it. Without occlusion the runtime dispatches a click/hover to *every* element
   * under the cursor (no `pointer-events` cascade), so overlapping elements all
   * receive it.
   *
   * Defaults to `true` for out-of-flow overlays (`position: "absolute"` or
   * `floating`) and `false` otherwise. Set it explicitly to decouple occlusion
   * from layout: an in-flow element can block events (e.g. a centred dialog panel
   * so clicks on it don't reach a dismiss layer behind), or an absolute element
   * can opt out (`occlude={false}`, e.g. a decorative scrim that lets clicks pass
   * through). Occlude sparingly — an occluding child suppresses an ancestor's
   * `_hover` / `_active` where it overlaps.
   */
  occlude?: boolean;
}

export interface TextProps extends EventProps {
  style?: StyleProps;
  children?: React.ReactNode;
}

export interface ImageProps extends EventProps {
  /**
   * Image source. Three forms accepted:
   *
   * - **Bundled asset**: static `import` — the Vite plugin prefixes with `asset://` so
   *   the runtime can locate the file in the compiled binary.
   * - **Remote URL**: `https://` / `http://` — requires the `http` Cargo feature;
   *   without it the image silently fails to load.
   * - **Local path**: `file://` URL or bare path (relative to the process cwd).
   *   Prefer absolute `file://` URLs for portability (handles Windows drive letters).
   */
  src: string;
  style?: StyleProps;
  /** Whether this image blocks the mouse from reaching elements behind it.
   *  Defaults to `true` for overlays (`position: "absolute"` / `floating`),
   *  `false` otherwise. See {@link ViewProps.occlude}. */
  occlude?: boolean;
}

export interface TextInputProps {
  value?: string;
  placeholder?: string;
  /** Accept multiple lines. Enter inserts a newline; `Cmd`/`Ctrl`+`Enter` fires
   *  {@link onSubmit}. The box grows with its content (soft-wrapping at its
   *  width), bounded by {@link minRows} / {@link maxRows}. */
  multiline?: boolean;
  /** `multiline` only: minimum number of visible rows (the auto-grow floor).
   *  Defaults to `1`. Ignored for single-line inputs. */
  minRows?: number;
  /** `multiline` only: maximum number of visible rows before the content
   *  scrolls internally. Omit for unbounded growth. Ignored for single-line. */
  maxRows?: number;
  /** Called with the new text on every keystroke (React Native-style). */
  onChangeText?: TextChangeHandler;
  /** Called with the current text on submit: `Enter` for single-line inputs,
   *  `Cmd`/`Ctrl`+`Enter` when `multiline`. */
  onSubmit?: TextChangeHandler;
  onFocus?: (e: GpuiFocusEvent) => void;
  onBlur?: (e: GpuiFocusEvent) => void;
  style?: StyleProps;
  /** Tab order index. A `<TextInput>` is keyboard-reachable by default;
   *  set `tabIndex={-1}` (or `tabStop={false}`) to remove it from the Tab order. */
  tabIndex?: number;
  /** Override whether this input is a Tab stop (default: `true`). */
  tabStop?: boolean;
  /** Ref to this element's {@link GpuiInstance} (`ref.current.focus()` / `.blur()`). */
  ref?: React.Ref<GpuiInstance>;
}

// With jsx: "react-jsx", TypeScript resolves intrinsics through React.JSX,
// so the augmentation must target the "react" module (not the global JSX namespace).
declare module "react" {
  namespace JSX {
    interface IntrinsicElements {
      View: ViewProps;
      Text: TextProps;
      Image: ImageProps;
      TextInput: TextInputProps;
    }
  }
}
