import { type GpuiMouseEvent, View, type ViewProps } from "@gluxe/react";
import React, { useCallback, useMemo } from "react";

import { Button } from "../button/button";
import { composeEventHandlers } from "../internal/compose";
import { createSafeContext } from "../internal/context";
import { useControllableState } from "../internal/controllable-state";
import { renderSlot, type Slot } from "../internal/slot";

/** State a Switch part exposes to its render-function children. */
export interface SwitchState {
  checked: boolean;
  disabled: boolean;
}

interface SwitchContextValue extends SwitchState {
  toggle: () => void;
}

const [SwitchContextProvider, useSwitchContext] = createSafeContext<SwitchContextValue>("Switch");

export interface SwitchProps extends Omit<ViewProps, "children"> {
  /** Controlled checked state. */
  checked?: boolean;
  /** Initial checked state when uncontrolled. */
  defaultChecked?: boolean;
  /** Called with the next checked state when the switch is toggled. */
  onCheckedChange?: (checked: boolean) => void;
  /** Prevents the switch from being toggled. */
  disabled?: boolean;
  children?: Slot<SwitchState>;
}

/**
 * A boolean on/off toggle. Wraps a {@link SwitchThumb} that represents the
 * sliding knob; consumers position and animate it by reading `checked` from
 * render-function children.
 *
 * Headless: no styles are applied. Read state via render-function children to
 * style by state, e.g. `<Switch>{({ checked }) => …}</Switch>`.
 *
 * Focusable via Tab and toggled with Space or Enter (the runtime activates a
 * focused control's click handler); a `disabled` switch is removed from the Tab
 * order. Style the focused state with `_focus` / `_focusVisible`.
 */
export function Switch({
  checked: checkedProp,
  defaultChecked,
  onCheckedChange,
  disabled = false,
  children,
  onClick,
  ...viewProps
}: SwitchProps): React.ReactElement {
  const [checked = false, setChecked] = useControllableState({
    prop: checkedProp,
    defaultProp: defaultChecked,
    onChange: onCheckedChange,
  });

  // No disabled guard: <Button> suppresses onClick while disabled.
  const toggle = useCallback(() => setChecked(!checked), [checked, setChecked]);

  const context = useMemo<SwitchContextValue>(
    () => ({ checked, disabled, toggle }),
    [checked, disabled, toggle],
  );

  return (
    <SwitchContextProvider value={context}>
      <Button
        {...viewProps}
        disabled={disabled}
        onClick={composeEventHandlers<GpuiMouseEvent>(onClick, toggle)}
      >
        {renderSlot(children, { checked, disabled })}
      </Button>
    </SwitchContextProvider>
  );
}
Switch.displayName = "Switch";

export interface SwitchThumbProps extends Omit<ViewProps, "children"> {
  children?: Slot<SwitchState>;
}

/**
 * The sliding knob of a {@link Switch}. Always renders regardless of the
 * checked state — consumers use render-function children or style props to
 * animate or reposition it. Must be rendered inside a `<Switch>`.
 */
export function SwitchThumb({ children, ...viewProps }: SwitchThumbProps): React.ReactElement {
  const { checked, disabled } = useSwitchContext();

  return <View {...viewProps}>{renderSlot(children, { checked, disabled })}</View>;
}
SwitchThumb.displayName = "Switch.Thumb";

Switch.Thumb = SwitchThumb;
