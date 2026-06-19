import { createContext, type Provider, useContext } from "react";

/**
 * Create a strict context bound to a compound component.
 *
 * Returns a `Provider` that takes the value directly and a hook that throws a
 * descriptive error when a part is rendered outside its root — the usual cause
 * of a confusing `null` deref in compound components.
 */
export function createSafeContext<T>(
  /** Name of the root component, used in the error message (e.g. `"Tabs"`). */
  rootName: string,
): readonly [Provider<T | null>, () => T] {
  const Context = createContext<T | null>(null);
  Context.displayName = `${rootName}Context`;

  function useSafeContext(): T {
    const value = useContext(Context);
    if (value === null) {
      throw new Error(`This component must be rendered inside <${rootName}>.`);
    }
    return value;
  }

  return [Context.Provider, useSafeContext] as const;
}
