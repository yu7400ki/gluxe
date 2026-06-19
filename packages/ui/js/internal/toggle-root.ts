import { useCallback } from "react";

import { useControllableState } from "./controllable-state";

/** Boolean toggle step: the stable `next` for on/off controls. */
export const negate = (current: boolean): boolean => !current;

export interface UseToggleRootParams<Stored, Change extends Stored = Stored> {
  /** Controlled value. When defined, the control is controlled. */
  prop: Stored | undefined;
  /** Initial value when uncontrolled. */
  defaultProp: Stored | undefined;
  /** Notified with the toggled-to value, in both modes. */
  onChange?: (value: Change) => void;
  /** Value while uncontrolled and no `defaultProp` was given. */
  defaultValue: Stored;
  /** The value to set when toggled, computed from the current value. Must have a
   *  stable identity (define it at module scope) so `toggle` stays stable. */
  next: (current: Stored) => Change;
}

/**
 * Shared root state for the boolean-ish toggles — Switch, Toggle, Disclosure,
 * Checkbox. Bridges controlled/uncontrolled state (via {@link useControllableState})
 * and returns the current value plus a `toggle` that sets `next(current)`.
 *
 * The components differ only in what they store and how a click advances it
 * (boolean negate, or tri-state → boolean for Checkbox), which `next` and the
 * `Stored` / `Change` type params capture. The surrounding context wiring stays
 * in each component since the value's name and context shape differ.
 */
export function useToggleRoot<Stored, Change extends Stored = Stored>({
  prop,
  defaultProp,
  onChange,
  defaultValue,
  next,
}: UseToggleRootParams<Stored, Change>): readonly [Stored, () => void] {
  const [value = defaultValue, setValue] = useControllableState<Stored, Change>({
    prop,
    defaultProp,
    onChange,
  });
  const toggle = useCallback(() => setValue(next(value)), [value, setValue, next]);
  return [value, toggle] as const;
}
