import { useCallback, useRef, useState } from "react";

export interface UseControllableStateParams<Stored, Change extends Stored = Stored> {
  /** The controlled value. When defined, the component is controlled. */
  prop: Stored | undefined;
  /** Initial value used while uncontrolled. */
  defaultProp: Stored | undefined;
  /** Notified with the value passed to the setter, in both controlled and
   *  uncontrolled modes. */
  onChange?: (value: Change) => void;
}

/**
 * Bridge controlled and uncontrolled state for a single value.
 *
 * When `prop` is defined the component is controlled: the setter only fires
 * `onChange` and never mutates internal state. Otherwise it tracks the value
 * internally (seeded from `defaultProp`) and still fires `onChange`.
 *
 * The stored value (`Stored`) and the value you ever *set* (`Change`) can
 * differ: the setter — and therefore `onChange` — accepts the narrower `Change`,
 * while `prop` / `defaultProp` / the returned value carry the wider `Stored`.
 * They coincide by default. Checkbox uses this: it stores `boolean |
 * "indeterminate"` but only ever toggles to a `boolean`, so `onChange` gets a
 * `boolean` directly instead of a hand-written narrowing.
 *
 * The returned value can be `undefined` when neither `prop` nor `defaultProp`
 * is supplied — callers should coalesce to a sensible default.
 */
export function useControllableState<Stored, Change extends Stored = Stored>({
  prop,
  defaultProp,
  onChange,
}: UseControllableStateParams<Stored, Change>): readonly [
  Stored | undefined,
  (next: Change) => void,
] {
  const [uncontrolled, setUncontrolled] = useState<Stored | undefined>(defaultProp);
  const isControlled = prop !== undefined;
  const value = isControlled ? prop : uncontrolled;

  // Keep onChange out of the setter's dependency list so the setter stays stable.
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  const setValue = useCallback(
    (next: Change) => {
      if (!isControlled) {
        setUncontrolled(next);
      }
      onChangeRef.current?.(next);
    },
    [isControlled],
  );

  return [value, setValue] as const;
}
