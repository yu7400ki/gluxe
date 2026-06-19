import type { GpuiInstance } from "@gluxe/react";
import {
  focusElement,
  focusFirstElement,
  getActiveElement,
  popTabScope,
  pushTabScope,
} from "@gluxe/react/focus";
import React, { useEffect } from "react";

// FocusScope: the shared focus primitive for modal overlays (Dialog, and future
// Popover / Drawer / Menu). On mount it focuses in, on unmount it restores, and
// (when `trapped`) it confines Tab via the core `pushTabScope`/`popTabScope`
// primitive — no sentinel guards, custom inner `tabIndex` works, Tab stays sync.

export interface FocusScopeProps {
  /**
   * Ref to the scoped element (e.g. the dialog panel). Root of the Tab scope and
   * the mount-focus target; should be a single focusable host element.
   */
  containerRef: React.RefObject<GpuiInstance | null>;
  /** Confine Tab to the scope while mounted. @default true */
  trapped?: boolean;
  /**
   * Override mount focus (default: focus the first focusable inside the scope),
   * e.g. to focus a specific field. Runs from the mount effect, so focus via
   * `ref.focus()` / `focusElement`, which tolerate a not-yet-painted target.
   */
  onMountAutoFocus?: () => void;
  /**
   * Override the unmount behaviour (default: restore focus to whatever was
   * focused before the scope mounted). Receives that previously-focused id.
   */
  onUnmountAutoFocus?: (previouslyFocused: number | null) => void;
  children: React.ReactNode;
}

/**
 * Scopes keyboard focus for an overlay: focuses in on mount, traps Tab while
 * mounted, restores focus on unmount. Renders its children unchanged.
 */
export function FocusScope({
  containerRef,
  trapped = true,
  onMountAutoFocus,
  onUnmountAutoFocus,
  children,
}: FocusScopeProps): React.ReactElement {
  // Trap + focus in on mount; release + restore on unmount. Runs once per open.
  useEffect(() => {
    const previouslyFocused = getActiveElement();
    const id = containerRef.current?.id;

    if (trapped && id != null) pushTabScope(id);

    if (onMountAutoFocus) onMountAutoFocus();
    else if (id != null) focusFirstElement(id);

    return () => {
      if (trapped && id != null) popTabScope(id);
      if (onUnmountAutoFocus) onUnmountAutoFocus(previouslyFocused);
      else if (previouslyFocused !== null) focusElement(previouslyFocused);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return <>{children}</>;
}
FocusScope.displayName = "FocusScope";
