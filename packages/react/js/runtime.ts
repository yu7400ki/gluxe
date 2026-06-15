// Framework bootstrap — analogous to Expo's registerRootComponent.

import React from "react";
import Reconciler from "react-reconciler";

import hostConfig from "./host-config";

type ReconcilerFactory = (config: typeof hostConfig) => ReturnType<typeof Reconciler>;

const createReconciler = Reconciler as unknown as ReconcilerFactory;
const rootContainer = { root: true } as const;

let _reconciler: ReturnType<typeof Reconciler> | null = null;

function getReconciler() {
  if (!_reconciler) {
    _reconciler = createReconciler(hostConfig);
  }
  return _reconciler;
}

/** Mount a React component as the root of the gluxe window. Call exactly once from the entry file. */
export function registerRootComponent(AppComponent: React.ComponentType<unknown>): void {
  const reconciler = getReconciler();

  // LegacyRoot (tag=0): synchronous initial mount — safest under Boa where
  // MessageChannel semantics differ from a browser.
  const container = reconciler.createContainer(
    rootContainer, // must be non-null for HostRoot deletions
    0, // tag: LegacyRoot
    null, // hydrationCallbacks
    false, // isStrictMode
    null, // concurrentUpdatesByDefaultOverride
    "", // identifierPrefix
    (err: unknown) => {
      console.error("Uncaught React error:", String(err));
    },
    (err: unknown) => {
      console.error("Caught React error:", String(err));
    },
    (err: unknown) => {
      console.error("React recoverable error:", String(err));
    },
    () => {},
  );

  reconciler.updateContainer(React.createElement(AppComponent, null), container, null, null);
}

/**
 * Render `children` into the window root rather than the caller's tree position
 * (gluxe's `createPortal`). Children become siblings of the app's root content —
 * escaping ancestor layout, overflow clipping, and stacking — while React state,
 * context, and handlers stay intact. For modals/dialogs/toasts; pair with
 * `position: "absolute", inset: 0` for a full-window overlay.
 */
export function createPortal(children: React.ReactNode, key?: string): React.ReactPortal {
  // Share the single root container with the app's main content (as react-dom
  // portals share `document.body`); `clearContainer` is HostRoot-only, so this
  // never wipes app content. Cast bridges react-reconciler's `ReactPortal` to
  // React's — the runtime value is a valid portal either way.
  return getReconciler().createPortal(
    children,
    rootContainer,
    null,
    key ?? null,
  ) as unknown as React.ReactPortal;
}
