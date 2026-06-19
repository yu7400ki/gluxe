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

/**
 * The value exposed on a host element `ref` (`ref.current`). Carries the
 * Rust-side element id plus imperative focus controls.
 *
 * @example
 * const ref = useRef<GpuiInstance>(null);
 * <View ref={ref} tabIndex={0} />;
 * ref.current?.focus();
 */
export interface GpuiInstance {
  /** Rust-side ElementId of this host element. */
  readonly id: number;
  /** Move keyboard focus to this element (no-op unless it is focusable). */
  focus(): Promise<void>;
  /** Remove keyboard focus from this element (only if it currently holds it). */
  blur(): Promise<void>;
}

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
  /** Positions this element as a floating overlay anchored to a named element. */
  floating?: FloatingProps;
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
}

export interface TextInputProps {
  value?: string;
  placeholder?: string;
  /** Called with the new text on every keystroke (React Native-style). */
  onChangeText?: TextChangeHandler;
  /** Called with the current text when Enter is pressed. */
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

type Px = `${number}px`;
type Rem = `${number}rem`;
type Percent = `${number}%`;

/** px, rem, %, auto, or bare number (treated as px). */
type Dimension = number | Px | Rem | Percent | "auto";
/** px, rem, %, or bare number (treated as px). `"auto"` is not valid. */
type LengthUnit = number | Px | Rem | Percent;
/** px, rem, or bare number (treated as px). `%` and `auto` are not valid. */
type AbsoluteUnit = number | Px | Rem;

/**
 * A CSS color string. The following formats are all accepted:
 *
 * - **Hex**: `"#rgb"`, `"#rgba"`, `"#rrggbb"`, `"#rrggbbaa"`
 *   (3-, 4-, 6-, and 8-digit; 4/8-digit include an alpha channel)
 * - **rgb / rgba**: `"rgb(255, 0, 0)"` / `"rgba(255, 0, 0, 0.5)"`
 *   (r/g/b in 0–255; alpha in 0.0–1.0)
 * - **hsl / hsla**: `"hsl(210, 50%, 40%)"` / `"hsla(210, 50%, 40%, 0.8)"`
 *   (hue in degrees 0–360; saturation and lightness in percent)
 * - **Named colors**: `"red"`, `"tomato"`, `"cornflowerblue"`, etc. (all 148 CSS Level 4 names)
 * - **`"transparent"`**: fully transparent
 */
export type Color = string;

/**
 * Tailwind-style box shadow presets.
 *
 * | Value  | Description |
 * |--------|-------------|
 * | `"none"` | No shadow |
 * | `"2xs"` | Extra-extra-small shadow (offset 1, no blur) |
 * | `"xs"` | Extra-small shadow (offset 1, blur 2) |
 * | `"sm"` | Small shadow (Tailwind `shadow-sm`) |
 * | `"md"` | Medium shadow (Tailwind `shadow-md`) |
 * | `"lg"` | Large shadow (Tailwind `shadow-lg`) |
 * | `"xl"` | Extra-large shadow (Tailwind `shadow-xl`) |
 * | `"2xl"` | 2× extra-large shadow (Tailwind `shadow-2xl`) |
 */
type BoxShadowPreset = "none" | "2xs" | "xs" | "sm" | "md" | "lg" | "xl" | "2xl";

/** A single CSS box-shadow layer. */
export interface BoxShadowValue {
  offsetX?: number;
  offsetY?: number;
  blurRadius?: number;
  spreadRadius?: number;
  /** Shadow colour. Default: black at ~10% opacity (matching Tailwind shadows). */
  color?: Color;
  /** Draws the shadow inside the element's border. */
  inset?: boolean;
}

/** CSS timing-function keyword for a style transition. */
export type TransitionEasing = "linear" | "ease" | "ease-in" | "ease-out" | "ease-in-out";

/**
 * A single transition declaration: animate changes to one (or all) animatable
 * style props over time, like CSS `transition`.
 *
 * Animatable props are lengths (`width`, `padding`, `margin`, `inset`/`top`/…,
 * `borderRadius`, `borderWidth`, `fontSize`, …), colors (`backgroundColor`,
 * `color`, `borderColor`, …), and scalars (`opacity`, `flexGrow`, `fontWeight`,
 * `aspectRatio`, …). Declarations naming a non-animatable prop (e.g.
 * `display`) are ignored. Length changes animate only between matching units
 * (px↔px, %↔%, rem↔rem); mismatched units, `"auto"`, and added/removed
 * values apply instantly.
 *
 * @example
 * style={{
 *   width: open ? 300 : 100,
 *   transition: { property: "all", duration: 200, easing: "ease" },
 * }}
 */
export interface TransitionValue {
  /**
   * `"all"` (every animatable prop) or the name of a single style prop.
   * Default: `"all"`. When several declarations match the same prop, a
   * specific name beats `"all"`, and the last declaration wins.
   */
  property?: "all" | keyof BaseStyleProps;
  duration: number;
  /** Default: `"ease"`. */
  easing?: TransitionEasing;
  /** Delay before the transition starts, in milliseconds. Default: `0`. */
  delay?: number;
}

/** Visual-only style fields. No pseudo-selector keys — prevents recursive nesting. */
export interface BaseStyleProps {
  display?: "flex" | "block" | "grid" | "none";
  /** CSS `flex` shorthand. A positive number sets `flexGrow` (React Native semantics:
   *  `flexShrink:1, flexBasis:0`). Keyword strings map to CSS presets. */
  flex?: number | "auto" | "initial" | "none";
  /** Overrides the numeric `flex` shorthand. */
  flexGrow?: number;
  /** Overrides the numeric `flex` shorthand. */
  flexShrink?: number;
  flexBasis?: Dimension;
  flexWrap?: "nowrap" | "wrap" | "wrap-reverse";
  flexDirection?: "row" | "column" | "row-reverse" | "column-reverse";
  width?: Dimension;
  height?: Dimension;
  minWidth?: Dimension;
  minHeight?: Dimension;
  maxWidth?: Dimension;
  maxHeight?: Dimension;
  /** Width ÷ height, e.g. `16/9`. */
  aspectRatio?: number;
  /** Uniform padding; per-axis/per-side props override. */
  padding?: LengthUnit;
  paddingX?: LengthUnit;
  paddingY?: LengthUnit;
  paddingTop?: LengthUnit;
  paddingRight?: LengthUnit;
  paddingBottom?: LengthUnit;
  paddingLeft?: LengthUnit;
  /** Uniform gap between flex/grid children; `gapX`/`gapY` override per axis. */
  gap?: LengthUnit;
  gapX?: LengthUnit;
  gapY?: LengthUnit;
  alignItems?: "center" | "flex-start" | "flex-end" | "baseline" | "stretch";
  alignSelf?: "flex-start" | "flex-end" | "center" | "baseline" | "stretch" | "start" | "end";
  /** Alignment of wrapped lines (requires `flexWrap`). */
  alignContent?:
    | "normal"
    | "center"
    | "flex-start"
    | "flex-end"
    | "start"
    | "end"
    | "space-between"
    | "space-around"
    | "space-evenly"
    | "stretch";
  justifyContent?:
    | "center"
    | "flex-start"
    | "flex-end"
    | "space-between"
    | "space-around"
    | "space-evenly";
  // ---- Grid ----
  /**
   * Number of equal-width columns. Requires `display: "grid"`.
   * **Note:** GPUI only supports uniform tracks (`repeat(N, minmax(0, 1fr))`).
   */
  gridTemplateColumns?: number;
  /** Number of equal-height rows. Requires `display: "grid"`. */
  gridTemplateRows?: number;
  /**
   * Column placement shorthand: number (start line), `"auto"`, `"span N"`, or `"A / B"`.
   * `gridColumnStart` / `gridColumnEnd` / `gridColumnSpan` override individual endpoints.
   */
  gridColumn?: number | string;
  /**
   * Row placement shorthand: number (start line), `"auto"`, `"span N"`, or `"A / B"`.
   * `gridRowStart` / `gridRowEnd` / `gridRowSpan` override individual endpoints.
   */
  gridRow?: number | string;
  /** Column start line. Negative values count from the end. Overrides `gridColumn` start. */
  gridColumnStart?: number;
  /** Column end line. Negative values count from the end. Overrides `gridColumn` end. */
  gridColumnEnd?: number;
  /** Overrides both `gridColumn` endpoints with a span. */
  gridColumnSpan?: number;
  /** Row start line. Negative values count from the end. Overrides `gridRow` start. */
  gridRowStart?: number;
  /** Row end line. Negative values count from the end. Overrides `gridRow` end. */
  gridRowEnd?: number;
  /** Overrides both `gridRow` endpoints with a span. */
  gridRowSpan?: number;
  /** Uniform margin. Supports `"auto"`. Per-axis/per-side props override. */
  margin?: Dimension;
  marginX?: Dimension;
  marginY?: Dimension;
  marginTop?: Dimension;
  marginRight?: Dimension;
  marginBottom?: Dimension;
  marginLeft?: Dimension;
  /** `"absolute"` takes the element out of flow; use `inset`/`top`/… to place it. */
  position?: "relative" | "absolute";
  /** Shorthand for all four inset offsets of a positioned element. */
  inset?: Dimension;
  top?: Dimension;
  right?: Dimension;
  bottom?: Dimension;
  left?: Dimension;
  /** Background color. Accepts any {@link Color} format.
   * @example `"#3d5a80"` · `"rgba(61,90,128,0.5)"` · `"hsl(210,50%,40%)"` · `"tomato"` · `"transparent"` */
  backgroundColor?: Color;
  borderRadius?: AbsoluteUnit;
  /** Pair with `borderColor` — GPUI only renders a border when both width and color are set. */
  borderWidth?: AbsoluteUnit;
  borderColor?: Color;
  color?: Color;
  fontSize?: AbsoluteUnit;
  /** `"bold"` = 700, `"normal"` = 400, or an explicit numeric value. */
  fontWeight?: "bold" | "normal" | number;
  cursor?:
    | "pointer"
    | "default"
    | "text"
    | "move"
    | "grab"
    | "grabbing"
    | "crosshair"
    | "not-allowed"
    | "no-drop"
    | "context-menu"
    | "copy"
    | "alias"
    | "vertical-text"
    | "ew-resize"
    | "ns-resize"
    | "nesw-resize"
    | "nwse-resize"
    | "col-resize"
    | "row-resize"
    | "n-resize"
    | "e-resize"
    | "s-resize"
    | "w-resize";
  /** Prevent text from wrapping to the next line. Combine with
   *  `textOverflow: "ellipsis"` to clip long text with `…`. */
  whiteSpace?: "nowrap" | "normal";
  /** How overflowing text is indicated. Only effective when
   *  `whiteSpace: "nowrap"` is also set.
   *  `"ellipsis-start"` truncates from the beginning (useful for paths). */
  textOverflow?: "ellipsis" | "clip" | "ellipsis-start";
  /** Clamp multi-line text to at most N lines. */
  lineClamp?: number;
  /**
   * Clip (and optionally scroll) content that overflows both axes.
   * `"scroll"` enables mouse-wheel scrolling; `"hidden"` clips silently.
   * Per-axis: use `overflowX` / `overflowY` to override.
   */
  overflow?: "visible" | "hidden" | "scroll";
  /** Per-axis overflow for the horizontal axis. Overrides `overflow`. */
  overflowX?: "visible" | "hidden" | "scroll";
  /** Per-axis overflow for the vertical axis. Overrides `overflow`. */
  overflowY?: "visible" | "hidden" | "scroll";
  // ---- Visual effects ----
  opacity?: number;
  /** Unlike `display: "none"`, a hidden element still occupies layout space. */
  visibility?: "visible" | "hidden";
  borderStyle?: "solid" | "dashed";
  borderTopWidth?: AbsoluteUnit;
  borderRightWidth?: AbsoluteUnit;
  borderBottomWidth?: AbsoluteUnit;
  borderLeftWidth?: AbsoluteUnit;
  borderTopLeftRadius?: AbsoluteUnit;
  borderTopRightRadius?: AbsoluteUnit;
  borderBottomRightRadius?: AbsoluteUnit;
  borderBottomLeftRadius?: AbsoluteUnit;
  /** Only effective when `overflow` / `overflowY` is `"scroll"`. */
  scrollbarWidth?: AbsoluteUnit;
  /**
   * Box shadow.
   *
   * Pass a preset name (`"sm"`, `"md"`, `"lg"`, etc.), a single shadow-layer
   * object, or an array of shadow-layer objects for multi-layer shadows.
   *
   * @example
   * ```tsx
   * // Preset
   * <View style={{ boxShadow: "lg" }} />
   * // Custom single layer
   * <View style={{ boxShadow: { offsetY: 4, blurRadius: 8, color: "#3d5a80aa" } }} />
   * // Multi-layer
   * <View style={{ boxShadow: [
   *   { offsetY: 2, blurRadius: 4, color: "#0000001a" },
   *   { offsetY: 6, blurRadius: 12, color: "#0000000d" },
   * ]}} />
   * ```
   */
  boxShadow?: BoxShadowPreset | BoxShadowValue | BoxShadowValue[];
  // ---- Text styling ----
  textAlign?: "left" | "center" | "right";
  fontStyle?: "normal" | "italic";
  /** `".SystemUIFont"` selects the platform UI font. */
  fontFamily?: string;
  /** Bare number = font-size multiplier (e.g. `1.5` = 1.5× fontSize). Also accepts px/rem/%. */
  lineHeight?: number | Px | Rem | Percent;
  /** Combine with a space for both: `"underline line-through"`. */
  textDecorationLine?: "none" | "underline" | "line-through" | "underline line-through";
  textDecorationColor?: Color;
  /** Only `"wavy"` has visual effect; applies to underlines only (strikethrough is always solid). */
  textDecorationStyle?: "solid" | "wavy";
  textDecorationThickness?: number | Px;
  /** Background drawn behind the text glyphs (highlight). */
  textBackgroundColor?: Color;
  /**
   * OpenType font features.
   *
   * Each key must be a 4-character ASCII alphanumeric feature tag (e.g. `"liga"`,
   * `"ss01"`). Values are:
   * - `true` / `false` — enable (1) or disable (0) the feature.
   * - A non-negative integer — pass the exact numeric value to the feature.
   *
   * @example `{ liga: false, ss01: true, cv01: 1 }`
   */
  fontFeatures?: Record<string, boolean | number>;
  /** `<TextInput>` caret colour. Defaults to the text `color`. Ignored on other elements. */
  caretColor?: Color;
  /** `<TextInput>` caret width (px/rem only; `%`/`auto` ignored). Defaults to `1px`. */
  caretWidth?: AbsoluteUnit;
  /** `<TextInput>` selection-highlight colour. Use a translucent colour so text
   *  stays legible. Defaults to a translucent blue. Ignored on other elements. */
  selectionColor?: Color;
}

export interface StyleProps extends BaseStyleProps {
  /** Applied when the pointer is hovering over the element (`:hover`). */
  _hover?: BaseStyleProps;
  /** Applied while the element is being pressed (`:active`). */
  _active?: BaseStyleProps;
  /** Applied while the element holds keyboard focus (`:focus`). Requires the
   *  element to be focusable (e.g. `tabIndex`). */
  _focus?: BaseStyleProps;
  /** Applied while the element is focused *via the keyboard* (`:focus-visible`).
   *  Prefer this over `_focus` for focus rings so they don't show on click. */
  _focusVisible?: BaseStyleProps;
  /**
   * Animate changes to animatable style props over time (CSS-like
   * transitions). Applies to prop changes only — `_hover`/`_active`
   * switching stays instant.
   */
  transition?: TransitionValue | TransitionValue[];
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
