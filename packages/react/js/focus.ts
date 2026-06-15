// Focus API — read the active element and move focus by raw ElementId.
//
// Pairs with the per-element `ref.current.focus()/blur()` (see GpuiInstance) for
// id-based control: save the active element before opening a modal, restore it on
// close. Building blocks for a userland focus trap / focus restoration; the core
// ships no trap policy of its own.

import { invoke } from "./invoke";

interface FocusBridge {
  /** Synchronous read of the focused element id (any kind), or null. */
  getActiveElement(): number | null;
}

const bridge = (globalThis as unknown as { __bridge: FocusBridge }).__bridge;

/**
 * The id of the element holding keyboard focus, or `null` if none.
 *
 * Synchronous (no Promise): returns focus as of the last paint, so call it before
 * moving focus (e.g. in a modal's mount effect, before focusing the dialog) to
 * capture the element to restore to later. Spans `<View>` / `<Image>` /
 * `<TextInput>`; the internal root fallback reads as `null`.
 *
 * @example
 * const prev = getActiveElement();
 * // … open modal, focus first field …
 * if (prev !== null) focusElement(prev); // on close
 */
export function getActiveElement(): number | null {
  return bridge.getActiveElement();
}

/**
 * Move keyboard focus to the element with this id. No-op if the id isn't
 * focusable or hasn't painted yet (a freshly-mounted target is retried for a few
 * frames). Use with a saved {@link getActiveElement} id to restore focus.
 */
export function focusElement(id: number): Promise<void> {
  return invoke<void>("__focus|focus", { id });
}

/** Remove keyboard focus from the element with this id (only if it holds focus). */
export function blurElement(id: number): Promise<void> {
  return invoke<void>("__focus|blur", { id });
}
