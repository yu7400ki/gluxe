import type { Ref, RefCallback } from "react";

/**
 * Combine several refs into one callback ref, so a component can attach its own
 * ref (e.g. for roving focus) without dropping a consumer-supplied `ref`.
 */
export function mergeRefs<T>(...refs: (Ref<T> | undefined)[]): RefCallback<T> {
  return (node: T | null) => {
    for (const ref of refs) {
      if (typeof ref === "function") {
        ref(node);
      } else if (ref) {
        (ref as { current: T | null }).current = node;
      }
    }
  };
}
