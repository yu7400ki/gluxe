import type { GpuiInstance, GpuiKeyboardEvent } from "@gluxe/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

// Roving tabindex: a group of items exposes a single Tab stop, and the arrow
// keys move focus between items (focus management is now possible — see the
// `tabIndex` / `ref.focus()` host APIs in @gluxe/react). Shared by RadioGroup
// (selection follows focus) and Tabs (automatic or manual activation).

/** Which arrow keys navigate. `"both"` (radios) responds to all four arrows. */
export type Orientation = "horizontal" | "vertical" | "both";

/** A navigation target derived from a key: a step, or jump to first/last. */
type NavDirection = -1 | 1 | "first" | "last" | null;

/**
 * Map a GPUI arrow/Home/End key to a navigation direction for `orientation`,
 * or `null` when the key does not navigate. Pure — covered by unit tests.
 */
export function arrowDirection(key: string, orientation: Orientation): NavDirection {
  if (key === "home") return "first";
  if (key === "end") return "last";
  const horizontal = orientation === "horizontal" || orientation === "both";
  const vertical = orientation === "vertical" || orientation === "both";
  if (horizontal && key === "left") return -1;
  if (horizontal && key === "right") return 1;
  if (vertical && key === "up") return -1;
  if (vertical && key === "down") return 1;
  return null;
}

/**
 * Index of the next non-disabled item from `from` stepping by `step`, skipping
 * disabled items and (when `loop`) wrapping past the ends. Returns `null` when
 * no enabled item is reachable. Pure — covered by unit tests.
 */
export function nextEnabledIndex(
  items: readonly { disabled: boolean }[],
  from: number,
  step: 1 | -1,
  loop: boolean,
): number | null {
  const n = items.length;
  if (n === 0) return null;
  let i = from;
  for (let count = 0; count < n; count++) {
    i += step;
    if (i < 0 || i >= n) {
      if (!loop) return null;
      i = (i + n) % n;
    }
    if (!items[i].disabled) return i;
  }
  return null;
}

/** A registered roving item. Fields stay live via a shared mutable object. */
export interface RovingItem {
  value: string;
  disabled: boolean;
  /** Move keyboard focus to this item's element. */
  focus: () => void;
}

/** The value of the first non-disabled item, or `undefined` if none. */
function firstEnabledValue(items: readonly RovingItem[]): string | undefined {
  const i = nextEnabledIndex(items, -1, 1, false);
  return i === null ? undefined : items[i].value;
}

export interface UseRovingFocusParams {
  orientation: Orientation;
  /** Wrap navigation past the ends. */
  loop: boolean;
  /** The "current" value (e.g. the selected one) that seeds the Tab stop. */
  value: string | undefined;
  /** Called when arrow navigation lands on a value (group decides whether to
   *  also select — RadioGroup selects). Omit for focus-only navigation, e.g.
   *  Tabs in manual-activation mode where Space / Enter selects instead. */
  onNavigate?: (value: string) => void;
}

export interface RovingFocus {
  /** Register an item; returns an unregister cleanup. Stable identity. */
  register: (item: RovingItem) => () => void;
  /** The value that should carry `tabIndex={0}` (others get `-1`). */
  tabbableValue: string | undefined;
  /** Notify that an item received focus (keeps the Tab stop in sync). */
  onItemFocus: (value: string) => void;
  /** Handle an arrow/Home/End key dispatched from the item with `value`. */
  onItemKeyDown: (value: string, e: GpuiKeyboardEvent) => void;
}

/**
 * Roving-tabindex state for a single-select group. The first enabled item
 * claims the Tab stop until a value is selected or focused.
 *
 * Navigation follows registration (mount) order, not visual order — fine for the
 * static lists these components target. Remounting an item moves it to the end,
 * so dynamic lists can diverge from on-screen order.
 */
export function useRovingFocus({
  orientation,
  loop,
  value,
  onNavigate,
}: UseRovingFocusParams): RovingFocus {
  const itemsRef = useRef<RovingItem[]>([]);
  const [tabbable, setTabbable] = useState<string | undefined>(value);

  // The selected value owns the Tab stop while enabled; a disabled selection
  // hands it to the first enabled item (so disabled items stay out of Tab order).
  useEffect(() => {
    if (value === undefined) return;
    const items = itemsRef.current;
    const selected = items.find((i) => i.value === value);
    setTabbable(selected && !selected.disabled ? value : firstEnabledValue(items));
  }, [value]);

  const register = useCallback((item: RovingItem) => {
    itemsRef.current.push(item);
    // The first enabled item claims the Tab stop while nothing is current.
    setTabbable((cur) => cur ?? (item.disabled ? undefined : item.value));
    return () => {
      itemsRef.current = itemsRef.current.filter((i) => i !== item);
      // If the tab stop unmounted, hand it to the first remaining enabled item.
      setTabbable((cur) => (cur === item.value ? firstEnabledValue(itemsRef.current) : cur));
    };
  }, []);

  const onItemFocus = useCallback((v: string) => setTabbable(v), []);

  const onItemKeyDown = useCallback(
    (v: string, e: GpuiKeyboardEvent) => {
      const dir = arrowDirection(e.key, orientation);
      if (dir === null) return;
      const items = itemsRef.current;
      const from = items.findIndex((i) => i.value === v);
      if (from < 0) return;

      let target: number | null;
      if (dir === "first") target = nextEnabledIndex(items, -1, 1, false);
      else if (dir === "last") target = nextEnabledIndex(items, items.length, -1, false);
      else target = nextEnabledIndex(items, from, dir, loop);
      if (target === null) return;

      const next = items[target];
      setTabbable(next.value);
      next.focus();
      onNavigate?.(next.value);
    },
    [orientation, loop, onNavigate],
  );

  return useMemo(
    () => ({ register, tabbableValue: tabbable, onItemFocus, onItemKeyDown }),
    [register, tabbable, onItemFocus, onItemKeyDown],
  );
}

/**
 * Register a roving item from inside an item component. Keeps the registered
 * record's fields live (so `disabled` / `value` changes are seen by navigation)
 * while registering the stable object exactly once.
 *
 * @returns a ref to attach to the item's host element (for `focus()`), plus the
 *   `tabIndex` it should carry and a key-down handler for arrow navigation.
 */
export function useRovingItem(
  roving: RovingFocus,
  value: string,
  disabled: boolean,
): {
  ref: React.RefObject<GpuiInstance | null>;
  tabIndex: number;
  onFocus: () => void;
  onKeyDown: (e: GpuiKeyboardEvent) => void;
} {
  const ref = useRef<GpuiInstance | null>(null);
  const itemRef = useRef<RovingItem>({
    value,
    disabled,
    focus: () => ref.current?.focus(),
  });
  itemRef.current.value = value;
  itemRef.current.disabled = disabled;

  const { register } = roving;
  useEffect(() => register(itemRef.current), [register]);

  return {
    ref,
    tabIndex: roving.tabbableValue === value ? 0 : -1,
    onFocus: () => roving.onItemFocus(value),
    onKeyDown: (e) => roving.onItemKeyDown(value, e),
  };
}
