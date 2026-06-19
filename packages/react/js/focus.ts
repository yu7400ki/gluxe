// Focus API — read the active element, move focus by raw ElementId, and confine
// Tab to a subtree (push/popTabScope).
//
// Pairs with the per-element `ref.current.focus()/blur()` (see GpuiInstance) for
// id-based control: save the active element before opening a modal, restore it on
// close. Building blocks for focus restoration and traps.

import { invoke } from "./invoke";

interface FocusBridge {
  /** Synchronous read of the focused element id (any kind), or null. */
  getActiveElement(): number | null;
  /** Synchronous read of the tab-stop focusable ids in a subtree, in Tab order. */
  getFocusableElements(rootId: number): number[];
  /** Confine Tab navigation to a subtree (push onto the scope stack). */
  pushTabScope(rootId: number): void;
  /** Release a Tab scope (remove from the scope stack by id). */
  popTabScope(rootId: number): void;
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

/**
 * Move focus to the first tab-stop focusable inside `rootId`'s subtree (or
 * `rootId` itself when it has none). The race-free way to focus into a freshly
 * opened scope (modal/popover): the runtime resolves the target *after* the mount
 * flush and retries until it paints — unlike {@link getFocusableElements} from a
 * mount effect, which reads the tree before the flush and sees nothing.
 */
export function focusFirstElement(rootId: number): Promise<void> {
  return invoke<void>("__focus|focusFirstIn", { id: rootId });
}

/**
 * The tab-stop focusable ids inside `rootId`'s subtree, in Tab order (ascending
 * `tabIndex`, ties by tree order). General-purpose query for custom focus logic.
 *
 * Synchronous, reading the tree as of the last paint, so call it after the
 * subtree has painted, not in the mount effect that creates it — queued mounts
 * aren't visible yet, giving `[]`.
 */
export function getFocusableElements(rootId: number): number[] {
  return bridge.getFocusableElements(rootId);
}

/**
 * Confine Tab / Shift+Tab to `rootId`'s subtree until {@link popTabScope} (the
 * runtime `inert`): Tab cycles the scope and can't reach outside it. Scopes stack;
 * programmatic focus ({@link focusElement}) is unaffected. Synchronous.
 */
export function pushTabScope(rootId: number): void {
  bridge.pushTabScope(rootId);
}

/** Release the Tab scope pushed for `rootId` (removes it by id). Synchronous. */
export function popTabScope(rootId: number): void {
  bridge.popTabScope(rootId);
}
