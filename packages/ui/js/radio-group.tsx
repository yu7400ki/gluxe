import { type GpuiKeyboardEvent, type GpuiMouseEvent, View, type ViewProps } from "@gluxe/react";
import React, { useMemo } from "react";

import { composeEventHandlers } from "./internal/compose";
import { createSafeContext } from "./internal/context";
import { useControllableState } from "./internal/controllable-state";
import { mergeRefs } from "./internal/merge-refs";
import { type RovingFocus, useRovingFocus, useRovingItem } from "./internal/roving-focus";
import { renderSlot, type Slot } from "./internal/slot";

/** State the RadioGroup root exposes to its render-function children. */
export interface RadioGroupState {
  value: string | undefined;
  disabled: boolean;
}

interface RadioGroupContextValue {
  value: string | undefined;
  setValue: (value: string) => void;
  disabled: boolean;
  roving: RovingFocus;
}

/** State a RadioGroup.Item (and its children) exposes to render-function children. */
export interface RadioItemState {
  checked: boolean;
  disabled: boolean;
  value: string;
}

interface RadioItemContextValue extends RadioItemState {}

const [RadioGroupContextProvider, useRadioGroupContext] =
  createSafeContext<RadioGroupContextValue>("RadioGroup");

const [RadioItemContextProvider, useRadioItemContext] =
  createSafeContext<RadioItemContextValue>("RadioGroup.Item");

export interface RadioGroupProps extends Omit<ViewProps, "children"> {
  /** Controlled selected value. */
  value?: string;
  /** Initial selected value when uncontrolled. */
  defaultValue?: string;
  /** Called with the newly selected item's value when it changes. */
  onValueChange?: (value: string) => void;
  /**
   * Disables all items in the group when `true`.
   * Individual items can still be disabled independently.
   * @default false
   */
  disabled?: boolean;
  children?: Slot<RadioGroupState>;
}

/**
 * A single-select radio group. Wraps any number of {@link RadioGroupItem} parts.
 *
 * Headless: no styles are applied. Read state via render-function children to
 * style by state, e.g. `<RadioGroup>{({ value }) => …}</RadioGroup>`.
 *
 * Supports both controlled (`value` + `onValueChange`) and uncontrolled
 * (`defaultValue`) usage.
 *
 * **Keyboard & focus:** the group is a single Tab stop (the selected item, or
 * the first enabled item when none is selected). Arrow keys (any direction)
 * move focus between items and select as they go (selection follows focus,
 * matching native radios), wrapping at the ends and skipping disabled items.
 * Home / End jump to the first / last item. Style focus with `_focusVisible`.
 */
export function RadioGroup({
  value: valueProp,
  defaultValue,
  onValueChange,
  disabled = false,
  children,
  ...viewProps
}: RadioGroupProps): React.ReactElement {
  const [value, setValue] = useControllableState<string>({
    prop: valueProp,
    defaultProp: defaultValue,
    onChange: onValueChange,
  });

  // Radios respond to all four arrows; selection follows focus.
  const roving = useRovingFocus({ orientation: "both", loop: true, value, onNavigate: setValue });

  const context = useMemo<RadioGroupContextValue>(
    () => ({ value, setValue, disabled, roving }),
    [value, setValue, disabled, roving],
  );

  return (
    <RadioGroupContextProvider value={context}>
      <View {...viewProps}>{renderSlot(children, { value, disabled })}</View>
    </RadioGroupContextProvider>
  );
}
RadioGroup.displayName = "RadioGroup";

export interface RadioGroupItemProps extends Omit<ViewProps, "children"> {
  /** This item's value; must be unique within the group. */
  value: string;
  /**
   * Disables this item independently of the group.
   * An item is effectively disabled when the group is disabled **or** this is `true`.
   * @default false
   */
  disabled?: boolean;
  children?: Slot<RadioItemState>;
}

/**
 * A single radio option inside a {@link RadioGroup}.
 *
 * Clicking a non-disabled item selects it (calls the group's `onValueChange`).
 * Use {@link RadioGroupIndicator} as a child to render a visible selection mark.
 *
 * Must be rendered inside `<RadioGroup>`.
 */
export function RadioGroupItem({
  value: itemValue,
  disabled: itemDisabled = false,
  children,
  onClick,
  onKeyDown,
  onFocus,
  ref,
  // Tab order is managed by roving focus; an explicit tabIndex is ignored.
  tabIndex: _tabIndex,
  ...viewProps
}: RadioGroupItemProps): React.ReactElement {
  const group = useRadioGroupContext();

  const checked = group.value === itemValue;
  const disabled = group.disabled || itemDisabled;
  const roving = useRovingItem(group.roving, itemValue, disabled);

  const handleClick = composeEventHandlers<GpuiMouseEvent>(onClick, () => {
    if (!disabled) {
      group.setValue(itemValue);
    }
  });

  // Arrow / Home / End only; Space/Enter selection comes from the runtime's click.
  const handleKeyDown = composeEventHandlers<GpuiKeyboardEvent>(onKeyDown, roving.onKeyDown);

  const handleFocus = composeEventHandlers(onFocus, roving.onFocus);

  const itemCtx = useMemo<RadioItemContextValue>(
    () => ({ checked, disabled, value: itemValue }),
    [checked, disabled, itemValue],
  );

  return (
    <RadioItemContextProvider value={itemCtx}>
      <View
        {...viewProps}
        ref={mergeRefs(ref, roving.ref)}
        tabIndex={roving.tabIndex}
        onClick={handleClick}
        onKeyDown={handleKeyDown}
        onFocus={handleFocus}
      >
        {renderSlot(children, itemCtx)}
      </View>
    </RadioItemContextProvider>
  );
}
RadioGroupItem.displayName = "RadioGroup.Item";

export interface RadioGroupIndicatorProps extends Omit<ViewProps, "children"> {
  children?: Slot<RadioItemState>;
}

/**
 * Visual selection indicator for a {@link RadioGroupItem}.
 *
 * Renders nothing when the parent item is not checked; mount a custom mark
 * (e.g. a filled circle) as children to make it visible.
 *
 * Must be rendered inside `<RadioGroup.Item>`.
 */
export function RadioGroupIndicator({
  children,
  ...viewProps
}: RadioGroupIndicatorProps): React.ReactElement | null {
  const itemState = useRadioItemContext();

  if (!itemState.checked) {
    return null;
  }

  return <View {...viewProps}>{renderSlot(children, itemState)}</View>;
}
RadioGroupIndicator.displayName = "RadioGroup.Indicator";

RadioGroup.Item = RadioGroupItem;
RadioGroup.Indicator = RadioGroupIndicator;
