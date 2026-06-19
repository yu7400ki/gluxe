import { type GpuiMouseEvent, View, type ViewProps } from "@gluxe/react";
import React, { useCallback, useMemo } from "react";

import { Button } from "../button/button";
import { composeEventHandlers } from "../internal/compose";
import { createSafeContext } from "../internal/context";
import { useControllableState } from "../internal/controllable-state";
import { renderSlot, type Slot } from "../internal/slot";

// ---------------------------------------------------------------------------
// Accordion-level context
// ---------------------------------------------------------------------------

interface AccordionContextValue {
  /** Returns true when the given item value is currently open. */
  isOpen: (itemValue: string) => boolean;
  /** Toggle the open state of the given item value. */
  toggle: (itemValue: string) => void;
  /** When true every item's trigger is inert. */
  disabled: boolean;
}

const [AccordionContextProvider, useAccordionContext] =
  createSafeContext<AccordionContextValue>("Accordion");

// ---------------------------------------------------------------------------
// Per-item context
// ---------------------------------------------------------------------------

/** State a per-item part exposes to its render-function children. */
export interface AccordionItemState {
  /** Whether this item is currently expanded. */
  open: boolean;
  /** Whether this item (or the whole accordion) is disabled. */
  disabled: boolean;
  /** The value key that identifies this item. */
  value: string;
}

interface AccordionItemContextValue extends AccordionItemState {}

const [AccordionItemContextProvider, useAccordionItemContext] =
  createSafeContext<AccordionItemContextValue>("Accordion.Item");

// ---------------------------------------------------------------------------
// Accordion props (discriminated union)
// ---------------------------------------------------------------------------

interface AccordionBaseProps extends Omit<ViewProps, "children"> {
  /** Disable all triggers in the accordion. Defaults to `false`. */
  disabled?: boolean;
  /** Item components to render. */
  children?: React.ReactNode;
}

/** Allows only one item to be open at a time. */
export interface AccordionSingleProps extends AccordionBaseProps {
  type: "single";
  /** Controlled open-item value. `undefined` means none are open. */
  value?: string;
  /** Initial open-item value when uncontrolled. */
  defaultValue?: string;
  /** Called with the item value whenever the open item changes. */
  onValueChange?: (value: string) => void;
  /**
   * When `false` (default) clicking the open item does nothing — at least one
   * item stays open once one has been opened. Set to `true` to allow the open
   * item to be closed.
   */
  collapsible?: boolean;
}

/** Allows any number of items to be open simultaneously. */
export interface AccordionMultipleProps extends AccordionBaseProps {
  type: "multiple";
  /** Controlled set of open-item values. */
  value?: string[];
  /** Initial set of open-item values when uncontrolled. */
  defaultValue?: string[];
  /** Called with the new set of open-item values after every toggle. */
  onValueChange?: (value: string[]) => void;
}

export type AccordionProps = AccordionSingleProps | AccordionMultipleProps;

// ---------------------------------------------------------------------------
// Accordion (root)
// ---------------------------------------------------------------------------

/** The open-state channel an Accordion mode exposes through the shared context. */
interface AccordionMode {
  isOpen: (itemValue: string) => boolean;
  toggle: (itemValue: string) => void;
}

/**
 * Single-mode state: at most one item open. `collapsible` lets a click close the
 * open item. Always called (hook order must be stable); when the accordion is in
 * multiple mode `props.type !== "single"`, so the controllable state idles.
 *
 * Narrowing `props` by `type` before reading `value` / `onValueChange` types them
 * as the single-mode shape (`string`) — no union casts.
 */
function useSingleAccordion(props: AccordionProps): AccordionMode {
  const single = props.type === "single" ? props : undefined;
  const [value, setValue] = useControllableState<string>({
    prop: single?.value,
    defaultProp: single?.defaultValue,
    onChange: single?.onValueChange,
  });
  const collapsible = single?.collapsible ?? false;

  const isOpen = useCallback((itemValue: string) => value === itemValue, [value]);
  const toggle = useCallback(
    (itemValue: string) => {
      if (value === itemValue) {
        // Already open — only close if collapsible. "" is the none-open sentinel;
        // isOpen's strict equality never matches a real item value.
        if (collapsible) {
          setValue("");
        }
      } else {
        setValue(itemValue);
      }
    },
    [value, collapsible, setValue],
  );
  return { isOpen, toggle };
}

/**
 * Multiple-mode state: any number of items open. Always called; idles when the
 * accordion is in single mode. Narrowing by `type` types `value` as `string[]`.
 */
function useMultipleAccordion(props: AccordionProps): AccordionMode {
  const multiple = props.type === "multiple" ? props : undefined;
  const [value = [], setValue] = useControllableState<string[]>({
    prop: multiple?.value,
    defaultProp: multiple?.defaultValue,
    onChange: multiple?.onValueChange,
  });

  const isOpen = useCallback((itemValue: string) => value.includes(itemValue), [value]);
  const toggle = useCallback(
    (itemValue: string) => {
      if (value.includes(itemValue)) {
        setValue(value.filter((v) => v !== itemValue));
      } else {
        setValue([...value, itemValue]);
      }
    },
    [value, setValue],
  );
  return { isOpen, toggle };
}

// Strip every Accordion control prop so only genuine `<View>` props reach the
// host element — leaking `value` / `onValueChange` / etc. would forward them
// natively. `collapsible` is single-mode only, so it is named via a single cast.
function accordionViewProps(props: AccordionProps): Omit<ViewProps, "children"> {
  const {
    type: _type,
    disabled: _disabled,
    value: _value,
    defaultValue: _defaultValue,
    onValueChange: _onValueChange,
    children: _children,
    collapsible: _collapsible,
    ...viewProps
  } = props as AccordionSingleProps;
  return viewProps;
}

/**
 * Expandable list of items. Each child should be an {@link AccordionItem}.
 *
 * - `type="single"` — only one item can be open at a time.
 * - `type="multiple"` — any number of items may be open simultaneously.
 *
 * Headless: no styles are applied. Discriminated `type` prop controls the
 * open-state shape; pass `collapsible` to allow closing the active single item.
 */
export function Accordion(props: AccordionProps): React.ReactElement {
  // Both mode hooks always run (stable hook order); `type` picks the active one.
  // Each narrows `props` internally, so there are no discriminated-union casts.
  const single = useSingleAccordion(props);
  const multiple = useMultipleAccordion(props);
  const { isOpen, toggle } = props.type === "single" ? single : multiple;

  const disabled = props.disabled ?? false;
  const ctx = useMemo<AccordionContextValue>(
    () => ({ isOpen, toggle, disabled }),
    [isOpen, toggle, disabled],
  );

  return (
    <AccordionContextProvider value={ctx}>
      <View {...accordionViewProps(props)}>{props.children}</View>
    </AccordionContextProvider>
  );
}
Accordion.displayName = "Accordion";

// ---------------------------------------------------------------------------
// AccordionItem
// ---------------------------------------------------------------------------

export interface AccordionItemProps extends Omit<ViewProps, "children"> {
  /** Unique identifier for this item; used to track open state. */
  value: string;
  /** Disable only this item's trigger. Stacks with the accordion-level flag. */
  disabled?: boolean;
  children?: Slot<AccordionItemState>;
}

/**
 * A single collapsible entry inside an {@link Accordion}. Must supply a unique
 * `value` string. Wrap a {@link AccordionTrigger} and an {@link AccordionContent}.
 */
export function AccordionItem({
  value: itemValue,
  disabled: itemDisabled = false,
  children,
  ...viewProps
}: AccordionItemProps): React.ReactElement {
  const accordionCtx = useAccordionContext();
  const open = accordionCtx.isOpen(itemValue);
  const disabled = accordionCtx.disabled || itemDisabled;

  const itemCtx = useMemo<AccordionItemContextValue>(
    () => ({ open, disabled, value: itemValue }),
    [open, disabled, itemValue],
  );

  return (
    <AccordionItemContextProvider value={itemCtx}>
      <View {...viewProps}>{renderSlot(children, itemCtx)}</View>
    </AccordionItemContextProvider>
  );
}
AccordionItem.displayName = "Accordion.Item";

// ---------------------------------------------------------------------------
// AccordionTrigger
// ---------------------------------------------------------------------------

export interface AccordionTriggerProps extends Omit<ViewProps, "children"> {
  children?: Slot<AccordionItemState>;
}

/**
 * The pressable header of an {@link AccordionItem}. Toggles the item open or
 * closed on click, or with Space / Enter while focused (the runtime activates a
 * focused control's click handler); does nothing when the item or accordion is
 * disabled. Each header is focusable via Tab (`tabIndex={0}`); a disabled header
 * leaves the Tab order.
 */
export function AccordionTrigger({
  children,
  onClick,
  ...viewProps
}: AccordionTriggerProps): React.ReactElement {
  const accordionCtx = useAccordionContext();
  const itemState = useAccordionItemContext();

  return (
    <Button
      {...viewProps}
      disabled={itemState.disabled}
      onClick={composeEventHandlers<GpuiMouseEvent>(onClick, () =>
        accordionCtx.toggle(itemState.value),
      )}
    >
      {renderSlot(children, itemState)}
    </Button>
  );
}
AccordionTrigger.displayName = "Accordion.Trigger";

// ---------------------------------------------------------------------------
// AccordionContent
// ---------------------------------------------------------------------------

export interface AccordionContentProps extends Omit<ViewProps, "children"> {
  children?: Slot<AccordionItemState>;
}

/**
 * The collapsible body of an {@link AccordionItem}. Renders nothing while the
 * item is closed, mounting its children only when open.
 */
export function AccordionContent({
  children,
  ...viewProps
}: AccordionContentProps): React.ReactElement | null {
  const itemState = useAccordionItemContext();

  if (!itemState.open) {
    return null;
  }

  return <View {...viewProps}>{renderSlot(children, itemState)}</View>;
}
AccordionContent.displayName = "Accordion.Content";

// ---------------------------------------------------------------------------
// Compound attachment
// ---------------------------------------------------------------------------

Accordion.Item = AccordionItem;
Accordion.Trigger = AccordionTrigger;
Accordion.Content = AccordionContent;
