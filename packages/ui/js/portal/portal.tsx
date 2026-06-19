import { createPortal } from "@gluxe/react";
import type React from "react";

export interface PortalProps {
  /** Content rendered into the window root. */
  children?: React.ReactNode;
}

/**
 * Renders its children into the window root instead of at its position in the
 * tree — escaping ancestor layout, overflow clipping, and stacking — while React
 * context, state, and event handlers stay intact. The declarative form of
 * `createPortal` from `@gluxe/react`.
 *
 * Portaled children become siblings of the app's root content and paint after
 * it, so they sit on top. Use it for overlays (dialogs, dropdowns, toasts); pair
 * the child with `position: "absolute", inset: 0` for a full-window layer.
 *
 * ```tsx
 * {open && (
 *   <Portal>
 *     <View style={{ position: "absolute", inset: 0 }} onClick={close} />
 *   </Portal>
 * )}
 * ```
 */
export function Portal({ children }: PortalProps): React.ReactPortal {
  return createPortal(children);
}
Portal.displayName = "Portal";
