import type { GpuiInstance, GpuiKeyboardEvent } from "@gluxe/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

// Keyboard navigation over a registry of items, in three flavours that share a
// registry and the arrow/Home/End math:
//   - Roving tabindex (`useRovingFocus`): the group exposes a single Tab stop
//     and arrows move it between items. Used by RadioGroup (selection follows
//     focus) and Tabs (automatic or manual activation).
//   - List navigation (`useListNavigation`): the Tab stop is external (a Select
//     trigger); arrows move a "highlight" over the open option list, with
//     explicit selection. Used by Select.
//   - Focus-group navigation (`useFocusGroupNavigation`): every item stays a Tab
//     stop (no roving tabindex); arrows / Home / End only MOVE focus between
//     items. Used by Accordion (the ARIA APG "all headers tabbable" pattern,
//     with arrow keys as an additive convenience).
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
  /** Text matched by Select type-ahead (case-insensitive prefix). Falls back to
   *  `value` when unset — set it when the visible label differs from `value`. */
  textValue?: string;
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

/** Rotate `arr` so element `start` comes first, wrapping the rest after the end. */
function wrapArray<T>(arr: readonly T[], start: number): T[] {
  return arr.map((_, i) => arr[(start + i) % arr.length]);
}

/**
 * The option to highlight for a type-ahead `buffer` (accumulated printable keys),
 * searching from `currentValue`, or `null` when nothing enabled matches. Pure —
 * covered by unit tests; the caller owns buffering / the idle reset.
 *
 * Mirrors the ARIA listbox type-ahead (Radix's `getNextMatch`):
 * - A buffer of the *same* char repeated (e.g. `"aa"`) matches just that char and
 *   advances past the current option, so repeats cycle through matches.
 * - A distinct multi-char buffer (e.g. `"ap"`) is matched as a full prefix from
 *   the current option, so a growing string refines in place.
 * Matching is case-insensitive on `textValue ?? value` and skips disabled options.
 * The search wraps (end → start) so any matching option is reachable.
 */
export function typeaheadMatch(
  items: readonly RovingItem[],
  currentValue: string | undefined,
  buffer: string,
): string | null {
  if (buffer === "") return null;
  const enabled = items.filter((i) => !i.disabled);
  if (enabled.length === 0) return null;

  const isRepeat = buffer.length > 1 && [...buffer].every((c) => c === buffer[0]);
  const search = (isRepeat ? buffer[0] : buffer).toLowerCase();

  // Rotate so the search starts at the current highlight (C2: fall back to 0 when
  // the highlight is unset or not found, so we never index out of range).
  const currentIndex = currentValue ? enabled.findIndex((i) => i.value === currentValue) : -1;
  let candidates = wrapArray(enabled, currentIndex < 0 ? 0 : currentIndex);

  // A single-char search advances to the NEXT match (drop the current option so a
  // press moves off it and repeats cycle); a multi-char prefix keeps the current
  // option as a candidate so refining can stay put.
  if (search.length === 1 && currentIndex >= 0) {
    candidates = candidates.filter((i) => i.value !== currentValue);
  }

  const match = candidates.find((i) => (i.textValue ?? i.value).toLowerCase().startsWith(search));
  return match ? match.value : null;
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

/** Idle window after the last printable key before the type-ahead buffer resets. */
const TYPEAHEAD_RESET_MS = 500;

/**
 * Highlight-channel navigation for a Select dropdown, over the shared item
 * registry. The highlight is internal state; options drive it via {@link useListItem}.
 */
export function useListNavigation({ loop }: { loop: boolean }): ListNavigation {
  const { register, items } = useItemRegistry();
  const [highlighted, setHighlighted] = useState<string | undefined>(undefined);

  // Type-ahead buffer + idle-reset timer. Refs so accumulating keys does not
  // re-render; the pure `typeaheadMatch` owns the matching, this owns timing.
  const buffer = useRef("");
  const resetTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearTypeahead = useCallback(() => {
    buffer.current = "";
    if (resetTimer.current !== null) {
      clearTimeout(resetTimer.current);
      resetTimer.current = null;
    }
  }, []);

  // C3: drop any pending buffer/timer when the component unmounts.
  useEffect(() => clearTypeahead, [clearTypeahead]);

  // The single highlight-move path shared by arrow nav AND type-ahead (C1): same
  // setter + same `focus()`, so the highlight and real keyboard focus never drift.
  const highlightAndFocus = useCallback(
    (item: RovingItem) => {
      setHighlighted(item.value);
      item.focus();
    },
    [],
  );

  // Wrap the exposed setter so clearing the highlight (list close / reset) also
  // drops the type-ahead buffer (C3) — the next open starts clean.
  const setHighlightedExternal = useCallback(
    (value: string | undefined) => {
      if (value === undefined) clearTypeahead();
      setHighlighted(value);
    },
    [clearTypeahead],
  );

  const onItemKeyDown = useCallback(
    (value: string, e: GpuiKeyboardEvent) => {
      const next = nextItemForKey(items.current, value, e.key, "vertical", loop);
      if (next) {
        highlightAndFocus(next);
        return;
      }
      // Type-ahead: a printable char with no command modifier (Shift is allowed —
      // matching is case-insensitive). `key.length === 1` excludes named keys
      // ("space", "enter", …) and IME / composed multi-byte input (out of scope).
      if (e.key.length === 1 && !e.ctrl && !e.alt && !e.meta) {
        if (resetTimer.current !== null) clearTimeout(resetTimer.current);
        buffer.current += e.key;
        resetTimer.current = setTimeout(() => {
          buffer.current = "";
          resetTimer.current = null;
        }, TYPEAHEAD_RESET_MS);

        // Search from the option that dispatched the key (the current highlight).
        const match = typeaheadMatch(items.current, value, buffer.current);
        if (match !== null) {
          const item = items.current.find((i) => i.value === match);
          if (item) highlightAndFocus(item);
        }
      }
    },
    [items, loop, highlightAndFocus],
  );

  const focusInitial = useCallback(
    (preferred: string | undefined) => {
      const list = items.current;
      const selected = list.find((i) => i.value === preferred && !i.disabled);
      const firstIndex = nextEnabledIndex(list, -1, 1, false);
      const target = selected ?? (firstIndex === null ? undefined : list[firstIndex]);
      if (!target) return;
      highlightAndFocus(target);
    },
    [items, highlightAndFocus],
  );

  return useMemo(
    () => ({
      register,
      highlighted,
      setHighlighted: setHighlightedExternal,
      onItemKeyDown,
      focusInitial,
    }),
    [register, highlighted, setHighlightedExternal, onItemKeyDown, focusInitial],
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
  textValue?: string,
): {
  ref: React.RefObject<GpuiInstance | null>;
  onFocus: () => void;
  onKeyDown: (e: GpuiKeyboardEvent) => void;
} {
  const ref = useRef<GpuiInstance | null>(null);
  const itemRef = useRef<RovingItem>({
    value,
    disabled,
    textValue,
    focus: () => ref.current?.focus(),
  });
  itemRef.current.value = value;
  itemRef.current.disabled = disabled;
  itemRef.current.textValue = textValue;

  const { register } = list;
  useEffect(() => register(itemRef.current), [register]);

  return {
    ref,
    onFocus: () => list.setHighlighted(value),
    onKeyDown: (e) => list.onItemKeyDown(value, e),
  };
}

// ---------------------------------------------------------------------------
// Focus-group navigation (Accordion): every item keeps its own Tab stop; arrow
// keys / Home / End only move focus, never touching tabindex or selection.
// ---------------------------------------------------------------------------

/**
 * Arrow / Home / End focus movement over a group whose items are *all* Tab stops
 * (the ARIA APG Accordion pattern). Unlike {@link useRovingFocus} it never
 * manages `tabIndex` and holds no React state — it only routes a key from the
 * focused item to the next/prev/first/last enabled item's `focus()`.
 */
export interface FocusGroupNavigation {
  /** Register an item; returns an unregister cleanup. Stable identity. */
  register: (item: RovingItem) => () => void;
  /** Handle an arrow / Home / End key dispatched from the item with `value`. */
  onItemKeyDown: (value: string, e: GpuiKeyboardEvent) => void;
}

/**
 * Focus-only group navigation, over the shared item registry. No `tabIndex`
 * management and no state: items stay Tab stops and arrows merely move focus.
 *
 * Navigation follows registration (mount) order, like the other flavours — fine
 * for the static lists these components target; an item mounted later registers
 * at the end, so a dynamically inserted header navigates from there.
 */
export function useFocusGroupNavigation({
  orientation,
  loop,
}: {
  orientation: Orientation;
  loop: boolean;
}): FocusGroupNavigation {
  const { register, items } = useItemRegistry();

  const onItemKeyDown = useCallback(
    (value: string, e: GpuiKeyboardEvent) => {
      const next = nextItemForKey(items.current, value, e.key, orientation, loop);
      if (next) next.focus();
    },
    [items, orientation, loop],
  );

  return useMemo(() => ({ register, onItemKeyDown }), [register, onItemKeyDown]);
}

/**
 * Register a focus-group item from inside an item component. Mirrors
 * {@link useListItem} minus the highlight channel: no `tabIndex` and no
 * `onFocus` — the item keeps whatever `tabIndex` it already has, and the only
 * wiring is registration plus an `onKeyDown` for arrow navigation.
 *
 * @returns a ref for the item's host element and an `onKeyDown` for arrow nav.
 */
export function useFocusGroupItem(
  group: FocusGroupNavigation,
  value: string,
  disabled: boolean,
): {
  ref: React.RefObject<GpuiInstance | null>;
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

  const { register } = group;
  useEffect(() => register(itemRef.current), [register]);

  return {
    ref,
    onKeyDown: (e) => group.onItemKeyDown(value, e),
  };
}
