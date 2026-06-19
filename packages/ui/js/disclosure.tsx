import { type GpuiMouseEvent, View, type ViewProps } from "@gluxe/react";
import React, { useCallback, useMemo } from "react";

import { composeEventHandlers } from "./internal/compose";
import { createSafeContext } from "./internal/context";
import { useControllableState } from "./internal/controllable-state";
import { renderSlot, type Slot } from "./internal/slot";

/** State a Disclosure part exposes to its render-function children. */
export interface DisclosureState {
  open: boolean;
  disabled: boolean;
}

interface DisclosureContextValue extends DisclosureState {
  toggle: () => void;
}

const [DisclosureContextProvider, useDisclosureContext] =
  createSafeContext<DisclosureContextValue>("Disclosure");

export interface DisclosureProps extends Omit<ViewProps, "children"> {
  /** Controlled open state. */
  open?: boolean;
  /** Initial open state when uncontrolled. */
  defaultOpen?: boolean;
  /** Called with the next open state when the trigger toggles. */
  onOpenChange?: (open: boolean) => void;
  /** Prevents the trigger from toggling. */
  disabled?: boolean;
  children?: Slot<DisclosureState>;
}

/**
 * An expand/collapse region (a.k.a. Collapsible). Wraps a {@link DisclosureTrigger}
 * and a {@link DisclosureContent}; the content mounts only while open.
 *
 * Headless: no styles are applied. Read state via render-function children to
 * style by state, e.g. `<Disclosure.Trigger>{({ open }) => …}</Disclosure.Trigger>`.
 */
export function Disclosure({
  open: openProp,
  defaultOpen,
  onOpenChange,
  disabled = false,
  children,
  ...viewProps
}: DisclosureProps): React.ReactElement {
  const [open = false, setOpen] = useControllableState({
    prop: openProp,
    defaultProp: defaultOpen,
    onChange: onOpenChange,
  });

  const toggle = useCallback(() => {
    if (!disabled) {
      setOpen(!open);
    }
  }, [disabled, open, setOpen]);

  const context = useMemo<DisclosureContextValue>(
    () => ({ open, disabled, toggle }),
    [open, disabled, toggle],
  );

  return (
    <DisclosureContextProvider value={context}>
      <View {...viewProps}>{renderSlot(children, { open, disabled })}</View>
    </DisclosureContextProvider>
  );
}
Disclosure.displayName = "Disclosure";

export interface DisclosureTriggerProps extends Omit<ViewProps, "children"> {
  children?: Slot<DisclosureState>;
}

/** Toggles the disclosure on click. Inherits all `<View>` props. */
export function DisclosureTrigger({
  children,
  onClick,
  ...viewProps
}: DisclosureTriggerProps): React.ReactElement {
  const { open, disabled, toggle } = useDisclosureContext();
  const handleClick = composeEventHandlers<GpuiMouseEvent>(onClick, () => toggle());

  return (
    <View {...viewProps} onClick={handleClick}>
      {renderSlot(children, { open, disabled })}
    </View>
  );
}
DisclosureTrigger.displayName = "Disclosure.Trigger";

export interface DisclosureContentProps extends Omit<ViewProps, "children"> {
  children?: Slot<DisclosureState>;
}

/** The collapsible region. Renders nothing while the disclosure is closed. */
export function DisclosureContent({
  children,
  ...viewProps
}: DisclosureContentProps): React.ReactElement | null {
  const { open, disabled } = useDisclosureContext();
  if (!open) {
    return null;
  }
  return <View {...viewProps}>{renderSlot(children, { open, disabled })}</View>;
}
DisclosureContent.displayName = "Disclosure.Content";

Disclosure.Trigger = DisclosureTrigger;
Disclosure.Content = DisclosureContent;
