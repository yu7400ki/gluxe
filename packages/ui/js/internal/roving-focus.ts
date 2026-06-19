import type { GpuiInstance, GpuiKeyboardEvent } from "@gluxe/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

// Keyboard navigation over a registry of items, in two flavours that share a
// registry and the arrow/Home/End math:
//   - Roving tabindex (`useRovingFocus`): the group exposes a single Tab stop
//     and arrows move it between items. Used by RadioGroup (selection follows
//     focus) and Tabs (automatic or manual activation).
//   - List navigation (`useListNavigation`): the Tab stop is external (a Select
//     trigger); arrows move a "highlight" over the open option list, with
//     explicit selection. Used by Select.
// Focus management is possible via the `tabIndex` / `ref.focus()` host APIs in
// @gluxe/react.

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

/**
 * A live registry of items: `register` appends (returning an unregister) and
 * `items` is the current list, kept in sync by mutation so callers always read
 * up-to-date `value` / `disabled`. The shared core of both navigation models —
 * the roving Tab stop and the Select highlight.
 */
function useItemRegistry(): {
  register: (item: RovingItem) => () => void;
  items: React.RefObject<RovingItem[]>;
} {
  const items = useRef<RovingItem[]>([]);
  const register = useCallback((item: RovingItem) => {
    items.current.push(item);
    return () => {
      items.current = items.current.filter((i) => i !== item);
    };
  }, []);
  return { register, items };
}

/**
 * The item to land on when navigating from `fromValue` with `key` under
 * `orientation`, or `null` when the key does not navigate or no enabled item is
 * reachable. Pure — wraps {@link arrowDirection} + {@link nextEnabledIndex}.
 */
function nextItemForKey(
  items: readonly RovingItem[],
  fromValue: string,
  key: string,
  orientation: Orientation,
  loop: boolean,
): RovingItem | null {
  const dir = arrowDirection(key, orientation);
  if (dir === null) return null;
  const from = items.findIndex((i) => i.value === fromValue);
  if (from < 0) return null;
  let target: number | null;
  if (dir === "first") target = nextEnabledIndex(items, -1, 1, false);
  else if (dir === "last") target = nextEnabledIndex(items, items.length, -1, false);
  else target = nextEnabledIndex(items, from, dir, loop);
  return target === null ? null : items[target];
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
  const { register: registerItem, items } = useItemRegistry();
  const [tabbable, setTabbable] = useState<string | undefined>(value);

  // The selected value owns the Tab stop while enabled; a disabled selection
  // hands it to the first enabled item (so disabled items stay out of Tab order).
  useEffect(() => {
    if (value === undefined) return;
    const selected = items.current.find((i) => i.value === value);
    setTabbable(selected && !selected.disabled ? value : firstEnabledValue(items.current));
  }, [value, items]);

  // Wrap registration with the Tab-stop bookkeeping: the first enabled item
  // claims the stop while nothing is current, and an unmounting stop hands off
  // to the first remaining enabled item.
  const register = useCallback(
    (item: RovingItem) => {
      const unregister = registerItem(item);
      setTabbable((cur) => cur ?? (item.disabled ? undefined : item.value));
      return () => {
        unregister();
        setTabbable((cur) => (cur === item.value ? firstEnabledValue(items.current) : cur));
      };
    },
    [registerItem, items],
  );

  const onItemFocus = useCallback((v: string) => setTabbable(v), []);

  const onItemKeyDown = useCallback(
    (v: string, e: GpuiKeyboardEvent) => {
      const next = nextItemForKey(items.current, v, e.key, orientation, loop);
      if (!next) return;
      setTabbable(next.value);
      next.focus();
      onNavigate?.(next.value);
    },
    [items, orientation, loop, onNavigate],
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

// ---------------------------------------------------------------------------
// List navigation (Select): a highlight channel over the same item registry.
// ---------------------------------------------------------------------------

/**
 * Keyboard navigation for a listbox-style popup (the Select dropdown). Unlike
 * {@link useRovingFocus} the Tab stop lives outside the list (on the trigger),
 * so options are never in the Tab order; a "highlight" instead tracks the
 * keyboard-focused option while the list is open, and selection is explicit
 * (click / Enter), not selection-follows-focus.
 */
export interface ListNavigation {
  /** Register an option; returns an unregister cleanup. Stable identity. */
  register: (item: RovingItem) => () => void;
  /** The highlighted (keyboard-focused) option's value while open, else undefined. */
  highlighted: string | undefined;
  /** Set the highlight (on focus, or `undefined` to clear when the list closes). */
  setHighlighted: (value: string | undefined) => void;
  /** Arrow / Home / End from option `value`: move the highlight and focus the new
   *  option (vertical only). Non-navigation keys (Escape, Enter) are the caller's. */
  onItemKeyDown: (value: string, e: GpuiKeyboardEvent) => void;
  /** Highlight and focus `preferred` if it is registered and enabled, otherwise
   *  the first enabled option. Called when the list opens. */
  focusInitial: (preferred: string | undefined) => void;
}

/**
 * Highlight-channel navigation for a Select dropdown, over the shared item
 * registry. The highlight is internal state; options drive it via {@link useListItem}.
 */
export function useListNavigation({ loop }: { loop: boolean }): ListNavigation {
  const { register, items } = useItemRegistry();
  const [highlighted, setHighlighted] = useState<string | undefined>(undefined);

  const onItemKeyDown = useCallback(
    (value: string, e: GpuiKeyboardEvent) => {
      const next = nextItemForKey(items.current, value, e.key, "vertical", loop);
      if (!next) return;
      setHighlighted(next.value);
      next.focus();
    },
    [items, loop],
  );

  const focusInitial = useCallback(
    (preferred: string | undefined) => {
      const list = items.current;
      const selected = list.find((i) => i.value === preferred && !i.disabled);
      const firstIndex = nextEnabledIndex(list, -1, 1, false);
      const target = selected ?? (firstIndex === null ? undefined : list[firstIndex]);
      if (!target) return;
      setHighlighted(target.value);
      target.focus();
    },
    [items],
  );

  return useMemo(
    () => ({ register, highlighted, setHighlighted, onItemKeyDown, focusInitial }),
    [register, highlighted, onItemKeyDown, focusInitial],
  );
}

/**
 * Register a Select option from inside an item component and wire its keyboard
 * focus. Mirrors {@link useRovingItem} for the highlight channel: there is no
 * `tabIndex` (options stay out of the Tab order) and focusing sets the highlight.
 *
 * @returns a ref for the option's host element, an `onFocus` that highlights it,
 *   and an `onKeyDown` for arrow navigation (Escape / Enter stay with the caller).
 */
export function useListItem(
  list: ListNavigation,
  value: string,
  disabled: boolean,
): {
  ref: React.RefObject<GpuiInstance | null>;
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

  const { register } = list;
  useEffect(() => register(itemRef.current), [register]);

  return {
    ref,
    onFocus: () => list.setHighlighted(value),
    onKeyDown: (e) => list.onItemKeyDown(value, e),
  };
}
