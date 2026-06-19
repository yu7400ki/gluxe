import { type GpuiKeyboardEvent, type GpuiMouseEvent, View, type ViewProps } from "@gluxe/react";
import React, { useCallback, useMemo } from "react";

import { composeEventHandlers } from "../internal/compose";
import { createSafeContext } from "../internal/context";
import { useControllableState } from "../internal/controllable-state";
import { mergeRefs } from "../internal/merge-refs";
import { type RovingFocus, useRovingFocus, useRovingItem } from "../internal/roving-focus";
import { renderSlot, type Slot } from "../internal/slot";

/** Orientation of the tab list, exposed via context for consumer styling. */
export type TabsOrientation = "horizontal" | "vertical";

/** Whether arrow-key focus also selects the tab (`"automatic"`) or only moves
 *  focus, deferring selection to Space / Enter (`"manual"`). */
export type TabsActivationMode = "automatic" | "manual";

interface TabsContextValue {
  value: string | undefined;
  setValue: (value: string) => void;
  orientation: TabsOrientation;
  roving: RovingFocus;
}

const [TabsContextProvider, useTabsContext] = createSafeContext<TabsContextValue>("Tabs");

export interface TabsProps extends Omit<ViewProps, "children"> {
  /** Controlled active tab value. */
  value?: string;
  /** Initial active tab value when uncontrolled. */
  defaultValue?: string;
  /** Called with the newly selected tab value when it changes. */
  onValueChange?: (value: string) => void;
  /**
   * Layout direction of the tab list. Also picks which arrows navigate:
   * Left/Right for `"horizontal"`, Up/Down for `"vertical"`.
   *
   * @default "horizontal"
   */
  orientation?: TabsOrientation;
  /**
   * Whether moving focus with the arrow keys also selects the tab.
   * - `"automatic"` (default) — selection follows focus.
   * - `"manual"` — arrows only move focus; Space / Enter selects.
   *
   * @default "automatic"
   */
  activationMode?: TabsActivationMode;
  /** Wrap arrow navigation past the first / last trigger. @default true */
  loop?: boolean;
  /** Compound-component children: `Tabs.List`, `Tabs.Trigger`, `Tabs.Content`. */
  children?: React.ReactNode;
}

/**
 * A tabbed-section root. Manages the active tab value (controlled or
 * uncontrolled) and exposes it to {@link TabsList}, {@link TabsTrigger}, and
 * {@link TabsContent} via context.
 *
 * Headless: no styles are applied. Style by state via render-function children
 * on each part, e.g.
 * `<Tabs.Trigger value="a">{({ selected }) => …}</Tabs.Trigger>`.
 *
 * **Keyboard navigation:** the tab list is a single Tab stop (the selected
 * trigger, or the first enabled trigger when none is selected). The arrow keys
 * for the `orientation` move focus between triggers — wrapping when `loop` —
 * and Home / End jump to the first / last. With `activationMode="automatic"`
 * (default) focus also selects; `"manual"` defers selection to Space / Enter.
 * Disabled triggers are skipped. Style focus with `_focusVisible`.
 */
export function Tabs({
  value: valueProp,
  defaultValue,
  onValueChange,
  orientation = "horizontal",
  activationMode = "automatic",
  loop = true,
  children,
  ...viewProps
}: TabsProps): React.ReactElement {
  const [value, setValue] = useControllableState<string>({
    prop: valueProp,
    defaultProp: defaultValue,
    onChange: onValueChange,
  });

  // Automatic activation selects on focus; manual only moves focus (stable noop
  // so the roving state doesn't churn its identity every render).
  const noop = useCallback(() => {}, []);
  const roving = useRovingFocus({
    orientation,
    loop,
    value,
    onNavigate: activationMode === "automatic" ? setValue : noop,
  });

  const context = useMemo<TabsContextValue>(
    () => ({ value, setValue, orientation, roving }),
    [value, setValue, orientation, roving],
  );

  return (
    <TabsContextProvider value={context}>
      <View {...viewProps}>{children}</View>
    </TabsContextProvider>
  );
}
Tabs.displayName = "Tabs";

export interface TabsListProps extends Omit<ViewProps, "children"> {
  /** `Tabs.Trigger` children. */
  children?: React.ReactNode;
}

/**
 * A container for {@link TabsTrigger} elements. Carries no built-in behavior;
 * use it as a layout wrapper and style it with the `style` prop.
 */
export function TabsList({ children, ...viewProps }: TabsListProps): React.ReactElement {
  return <View {...viewProps}>{children}</View>;
}
TabsList.displayName = "Tabs.List";

/** State exposed to `Tabs.Trigger` render-function children. */
export interface TabsTriggerState {
  /** Whether this trigger's tab is the active one. */
  selected: boolean;
  /** Whether this trigger is disabled. */
  disabled: boolean;
  /** The tab value associated with this trigger. */
  value: string;
}

export interface TabsTriggerProps extends Omit<ViewProps, "children"> {
  /** The tab value this trigger selects. Required. */
  value: string;
  /** When true, clicking this trigger does nothing. */
  disabled?: boolean;
  children?: Slot<TabsTriggerState>;
}

/**
 * A clickable trigger that activates the {@link TabsContent} with the matching
 * `value`. Must be rendered inside {@link TabsList}.
 *
 * Passes `{ selected, disabled, value }` to render-function children so
 * consumers can style the active/disabled states without any built-in CSS.
 * Keyboard-navigable via the roving Tab stop described on {@link Tabs}.
 */
export function TabsTrigger({
  value: itemValue,
  disabled = false,
  children,
  onClick,
  onKeyDown,
  onFocus,
  ref,
  // Tab order is managed by roving focus; an explicit tabIndex is ignored.
  tabIndex: _tabIndex,
  ...viewProps
}: TabsTriggerProps): React.ReactElement {
  const ctx = useTabsContext();
  const selected = ctx.value === itemValue;
  const roving = useRovingItem(ctx.roving, itemValue, disabled);

  const handleClick = composeEventHandlers<GpuiMouseEvent>(onClick, () => {
    if (!disabled) {
      ctx.setValue(itemValue);
    }
  });

  // Arrow / Home / End only; Space/Enter selection (incl. manual mode) comes from
  // the runtime's click.
  const handleKeyDown = composeEventHandlers<GpuiKeyboardEvent>(onKeyDown, roving.onKeyDown);

  const handleFocus = composeEventHandlers(onFocus, roving.onFocus);

  return (
    <View
      {...viewProps}
      ref={mergeRefs(ref, roving.ref)}
      tabIndex={roving.tabIndex}
      onClick={handleClick}
      onKeyDown={handleKeyDown}
      onFocus={handleFocus}
    >
      {renderSlot(children, { selected, disabled, value: itemValue })}
    </View>
  );
}
TabsTrigger.displayName = "Tabs.Trigger";

export interface TabsContentProps extends Omit<ViewProps, "children"> {
  /** The tab value this content panel belongs to. Required. */
  value: string;
  children?: Slot<{ selected: boolean }>;
}

/**
 * The content panel shown when its `value` matches the active tab. Renders
 * nothing while its tab is inactive — the panel is unmounted, not hidden.
 *
 * Passes `{ selected }` to render-function children (always `true` while
 * mounted, but available for API symmetry with other parts).
 */
export function TabsContent({
  value: itemValue,
  children,
  ...viewProps
}: TabsContentProps): React.ReactElement | null {
  const ctx = useTabsContext();
  const selected = ctx.value === itemValue;

  if (!selected) {
    return null;
  }

  return <View {...viewProps}>{renderSlot(children, { selected })}</View>;
}
TabsContent.displayName = "Tabs.Content";

Tabs.List = TabsList;
Tabs.Trigger = TabsTrigger;
Tabs.Content = TabsContent;
