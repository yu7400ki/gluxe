import { useCallback, useRef, useState } from "react";

export interface UseControllableStateParams<T> {
  /** The controlled value. When defined, the component is controlled. */
  prop: T | undefined;
  /** Initial value used while uncontrolled. */
  defaultProp: T | undefined;
  /** Notified whenever the value is set, in both controlled and uncontrolled modes. */
  onChange?: (value: T) => void;
}

/**
 * Bridge controlled and uncontrolled state for a single value.
 *
 * When `prop` is defined the component is controlled: the setter only fires
 * `onChange` and never mutates internal state. Otherwise it tracks the value
 * internally (seeded from `defaultProp`) and still fires `onChange`.
 *
 * The returned value can be `undefined` when neither `prop` nor `defaultProp`
 * is supplied — callers should coalesce to a sensible default.
 */
export function useControllableState<T>({
  prop,
  defaultProp,
  onChange,
}: UseControllableStateParams<T>): readonly [T | undefined, (next: T) => void] {
  const [uncontrolled, setUncontrolled] = useState<T | undefined>(defaultProp);
  const isControlled = prop !== undefined;
  const value = isControlled ? prop : uncontrolled;

  // Keep onChange out of the setter's dependency list so the setter stays stable.
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  const setValue = useCallback(
    (next: T) => {
      if (!isControlled) {
        setUncontrolled(next);
      }
      onChangeRef.current?.(next);
    },
    [isControlled],
  );

  return [value, setValue] as const;
}
