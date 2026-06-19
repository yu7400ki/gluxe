import { type GpuiMouseEvent, View, type ViewProps } from "@gluxe/react";
import React, { useMemo } from "react";

import { composeEventHandlers } from "./internal/compose";
import { createSafeContext } from "./internal/context";
import { useControllableState } from "./internal/controllable-state";
import { renderSlot, type Slot } from "./internal/slot";

/** Orientation of the tab list, exposed via context for consumer styling. */
export type TabsOrientation = "horizontal" | "vertical";

interface TabsContextValue {
  value: string | undefined;
  setValue: (value: string) => void;
  orientation: TabsOrientation;
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
   * Layout direction of the tab list.
   *
   * **Note:** GPUI has no programmatic focus API, so arrow-key roving-focus
   * navigation between triggers is not possible in this framework. Selection
   * occurs on click only. `orientation` is accepted and forwarded via context
   * so consumers can style the list direction themselves, but the library does
   * not implement keyboard navigation.
   *
   * @default "horizontal"
   */
  orientation?: TabsOrientation;
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
 * **Keyboard navigation:** GPUI has no programmatic focus API, so roving-focus
 * arrow-key navigation between triggers is not implemented. Tab selection
 * happens on click only. The `orientation` prop is still accepted and exposed
 * via context for consumer-driven styling.
 */
export function Tabs({
  value: valueProp,
  defaultValue,
  onValueChange,
  orientation = "horizontal",
  children,
  ...viewProps
}: TabsProps): React.ReactElement {
  const [value, setValue] = useControllableState<string>({
    prop: valueProp,
    defaultProp: defaultValue,
    onChange: onValueChange,
  });

  const context = useMemo<TabsContextValue>(
    () => ({ value, setValue, orientation }),
    [value, setValue, orientation],
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
 */
export function TabsTrigger({
  value: itemValue,
  disabled = false,
  children,
  onClick,
  ...viewProps
}: TabsTriggerProps): React.ReactElement {
  const ctx = useTabsContext();
  const selected = ctx.value === itemValue;

  const handleClick = composeEventHandlers<GpuiMouseEvent>(onClick, () => {
    if (!disabled) {
      ctx.setValue(itemValue);
    }
  });

  return (
    <View {...viewProps} onClick={handleClick}>
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
