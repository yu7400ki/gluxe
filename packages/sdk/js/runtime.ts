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
