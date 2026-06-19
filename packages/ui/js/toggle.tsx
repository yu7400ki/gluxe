import { type GpuiMouseEvent, View, type ViewProps } from "@gluxe/react";
import React from "react";

import { composeEventHandlers } from "./internal/compose";
import { useControllableState } from "./internal/controllable-state";
import { renderSlot, type Slot } from "./internal/slot";

/** State the Toggle exposes to its render-function children. */
export interface ToggleState {
  pressed: boolean;
  disabled: boolean;
}

export interface ToggleProps extends Omit<ViewProps, "children"> {
  /** Controlled pressed state. */
  pressed?: boolean;
  /** Initial pressed state when uncontrolled. */
  defaultPressed?: boolean;
  /** Called with the next pressed state when the toggle is clicked. */
  onPressedChange?: (pressed: boolean) => void;
  /** Prevents the toggle from changing state when clicked. Default `false`. */
  disabled?: boolean;
  children?: Slot<ToggleState>;
}

/**
 * A single pressable element with a boolean pressed state.
 *
 * Headless: no styles, no cursor, nothing — just behaviour. Read the current
 * state via render-function children to style by state, e.g.
 *
 * ```tsx
 * <Toggle>
 *   {({ pressed }) => (
 *     <View style={{ background: pressed ? "#3d5a80" : "#ccc" }}>Bold</View>
 *   )}
 * </Toggle>
 * ```
 *
 * ### Controlled vs. uncontrolled
 * - **Uncontrolled**: omit `pressed`; optionally supply `defaultPressed`.
 * - **Controlled**: supply `pressed` and update it in `onPressedChange`.
 *
 * ### Disabled
 * When `disabled` is `true` the click handler is swallowed before any state
 * change occurs. `onPressedChange` is never called.
 */
export function Toggle({
  pressed: pressedProp,
  defaultPressed,
  onPressedChange,
  disabled = false,
  children,
  onClick,
  ...viewProps
}: ToggleProps): React.ReactElement {
  const [pressed = false, setPressed] = useControllableState({
    prop: pressedProp,
    defaultProp: defaultPressed,
    onChange: onPressedChange,
  });

  const handleClick = composeEventHandlers<GpuiMouseEvent>(onClick, () => {
    if (!disabled) {
      setPressed(!pressed);
    }
  });

  return (
    <View {...viewProps} onClick={handleClick}>
      {renderSlot(children, { pressed, disabled })}
    </View>
  );
}
Toggle.displayName = "Toggle";
