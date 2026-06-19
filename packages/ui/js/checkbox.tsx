import { type GpuiMouseEvent, View, type ViewProps } from "@gluxe/react";
import React, { useCallback, useMemo } from "react";

import { composeEventHandlers } from "./internal/compose";
import { createSafeContext } from "./internal/context";
import { useControllableState } from "./internal/controllable-state";
import { renderSlot, type Slot } from "./internal/slot";

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
  // We intentionally omit onChange from useControllableState because its
  // signature is (value: CheckedState) => void, but onCheckedChange expects
  // only a boolean. We call onCheckedChange manually in the toggle wrapper.
  const [checkedState = false, setCheckedState] = useControllableState<CheckedState>({
    prop: checkedProp,
    defaultProp: defaultChecked,
  });

  const toggle = useCallback(() => {
    if (disabled) return;
    const next = checkedState !== true;
    setCheckedState(next);
    onCheckedChange?.(next);
  }, [disabled, checkedState, setCheckedState, onCheckedChange]);

  const context = useMemo<CheckboxContextValue>(
    () => ({ checked: checkedState, disabled, toggle }),
    [checkedState, disabled, toggle],
  );

  return (
    <CheckboxContextProvider value={context}>
      <View {...viewProps} onClick={composeEventHandlers<GpuiMouseEvent>(onClick, () => toggle())}>
        {renderSlot(children, { checked: checkedState, disabled })}
      </View>
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
