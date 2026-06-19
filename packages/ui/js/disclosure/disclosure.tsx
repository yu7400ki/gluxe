import { type GpuiMouseEvent, View, type ViewProps } from "@gluxe/react";
import React, { useMemo } from "react";

import { Button } from "../button/button";
import { composeEventHandlers } from "../internal/compose";
import { createSafeContext } from "../internal/context";
import { renderSlot, type Slot } from "../internal/slot";
import { negate, useToggleRoot } from "../internal/toggle-root";

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
  // No disabled guard: the trigger's <Button> suppresses onClick while disabled.
  const [open, toggle] = useToggleRoot({
    prop: openProp,
    defaultProp: defaultOpen,
    onChange: onOpenChange,
    defaultValue: false,
    next: negate,
  });

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

/**
 * Toggles the disclosure on click, or with Space / Enter while focused (the
 * runtime activates a focused control's click handler). Focusable via Tab
 * (`tabIndex={0}`); a `disabled` trigger leaves the Tab order. Inherits all
 * `<View>` props.
 */
export function DisclosureTrigger({
  children,
  onClick,
  ...viewProps
}: DisclosureTriggerProps): React.ReactElement {
  const { open, disabled, toggle } = useDisclosureContext();

  return (
    <Button
      {...viewProps}
      disabled={disabled}
      onClick={composeEventHandlers<GpuiMouseEvent>(onClick, toggle)}
    >
      {renderSlot(children, { open, disabled })}
    </Button>
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
