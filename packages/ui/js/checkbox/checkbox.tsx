import { type GpuiMouseEvent, View, type ViewProps } from "@gluxe/react";
import React, { useCallback, useMemo } from "react";

import { Button } from "../button/button";
import { composeEventHandlers } from "../internal/compose";
import { createSafeContext } from "../internal/context";
import { useControllableState } from "../internal/controllable-state";
import { renderSlot, type Slot } from "../internal/slot";

/** The three possible checked states of a {@link Checkbox}. */
export type CheckedState = boolean | "indeterminate";

/** State a Checkbox part exposes to its render-function children. */
export interface CheckboxState {
  checked: CheckedState;
  disabled: boolean;
}

interface CheckboxContextValue extends CheckboxState {
  toggle: () => void;
}

const [CheckboxContextProvider, useCheckboxContext] =
  createSafeContext<CheckboxContextValue>("Checkbox");

export interface CheckboxProps extends Omit<ViewProps, "children"> {
  /** Controlled checked state (`true`, `false`, or `"indeterminate"`). */
  checked?: CheckedState;
  /** Initial checked state when uncontrolled. */
  defaultChecked?: CheckedState;
  /**
   * Called with the **next boolean** state when the user toggles the checkbox.
   * Indeterminate counts as unchecked for the purpose of cycling: clicking an
   * indeterminate checkbox moves it to `true`.
   */
  onCheckedChange?: (checked: boolean) => void;
  /** Prevents toggling when `true`. Defaults to `false`. */
  disabled?: boolean;
  children?: Slot<CheckboxState>;
}

/**
 * A tri-state checkbox (`true` | `false` | `"indeterminate"`).
 *
 * Wraps a {@link CheckboxIndicator} that renders only when checked or
 * indeterminate. Use render-function children on any part to style by state:
 *
 * ```tsx
 * <Checkbox defaultChecked={false}>
 *   {({ checked, disabled }) => (
 *     <View style={{ opacity: disabled ? 0.4 : 1 }}>
 *       <Checkbox.Indicator>
 *         {({ checked }) => checked === "indeterminate" ? "–" : "✓"}
 *       </Checkbox.Indicator>
 *     </View>
 *   )}
 * </Checkbox>
 * ```
 *
 * Headless: no styles are applied. All `<View>` props are forwarded.
 *
 * Focusable via Tab and toggled with Space or Enter (the runtime activates a
 * focused control's click handler); a `disabled` checkbox is removed from the
 * Tab order. Style the focused state with `_focus` / `_focusVisible`.
 */
export function Checkbox({
  checked: checkedProp,
  defaultChecked,
  onCheckedChange,
  disabled = false,
  children,
  onClick,
  ...viewProps
}: CheckboxProps): React.ReactElement {
  // Stores a tri-state value but only ever toggles to a boolean, so the setter
  // (and onCheckedChange) take the narrower boolean directly. Indeterminate
  // counts as unchecked, so a click moves it to `true`.
  const [checkedState = false, setChecked] = useControllableState<CheckedState, boolean>({
    prop: checkedProp,
    defaultProp: defaultChecked,
    onChange: onCheckedChange,
  });

  // No disabled guard: <Button> suppresses onClick while disabled.
  const toggle = useCallback(() => setChecked(checkedState !== true), [checkedState, setChecked]);

  const context = useMemo<CheckboxContextValue>(
    () => ({ checked: checkedState, disabled, toggle }),
    [checkedState, disabled, toggle],
  );

  return (
    <CheckboxContextProvider value={context}>
      <Button
        {...viewProps}
        disabled={disabled}
        onClick={composeEventHandlers<GpuiMouseEvent>(onClick, toggle)}
      >
        {renderSlot(children, { checked: checkedState, disabled })}
      </Button>
    </CheckboxContextProvider>
  );
}
Checkbox.displayName = "Checkbox";

export interface CheckboxIndicatorProps extends Omit<ViewProps, "children"> {
  children?: Slot<CheckboxState>;
}

/**
 * Renders only when the checkbox is checked or indeterminate; returns `null`
 * when unchecked. Use render-function children to distinguish `true` from
 * `"indeterminate"` and render the appropriate visual:
 *
 * ```tsx
 * <Checkbox.Indicator>
 *   {({ checked }) => checked === "indeterminate" ? <Dash /> : <Check />}
 * </Checkbox.Indicator>
 * ```
 */
export function CheckboxIndicator({
  children,
  ...viewProps
}: CheckboxIndicatorProps): React.ReactElement | null {
  const { checked, disabled } = useCheckboxContext();
  if (checked === false) {
    return null;
  }
  return <View {...viewProps}>{renderSlot(children, { checked, disabled })}</View>;
}
CheckboxIndicator.displayName = "Checkbox.Indicator";

Checkbox.Indicator = CheckboxIndicator;
