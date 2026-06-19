import { View, type ViewProps } from "@gluxe/react";
import React from "react";

export interface ButtonProps extends ViewProps {
  /**
   * Removes the button from the Tab order and suppresses its `onClick`
   * (pointer and keyboard alike). It applies no visual style — read `disabled`
   * yourself to style it. Defaults to `false`.
   */
  disabled?: boolean;
}

/**
 * A headless pressable button: a focusable `<View>` whose `onClick` fires on a
 * pointer click **or** on Space / Enter while focused.
 *
 * Keyboard activation is provided by the GPUI runtime, which synthesizes a
 * click on any focused control that has a click handler — so `onClick` is the
 * single press callback for both mouse and keyboard, and there is no separate
 * key handler to wire up.
 *
 * Headless: no styles are applied, not even a cursor. It is focusable by default
 * (`tabIndex={0}`); pass an explicit `tabIndex` to override, or `disabled` to
 * leave the Tab order. Style the focused state with the `_focus` /
 * `_focusVisible` style props.
 *
 * ```tsx
 * <Button
 *   onClick={save}
 *   style={{ padding: 8, _focusVisible: { borderWidth: 2, borderColor: "#3d5a80" } }}
 * >
 *   <Text>Save</Text>
 * </Button>
 * ```
 */
export function Button({
  disabled = false,
  onClick,
  tabIndex,
  children,
  ...viewProps
}: ButtonProps): React.ReactElement {
  return (
    <View
      {...viewProps}
      tabIndex={tabIndex ?? (disabled ? undefined : 0)}
      onClick={disabled ? undefined : onClick}
    >
      {children}
    </View>
  );
}
Button.displayName = "Button";
