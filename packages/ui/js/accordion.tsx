import { type GpuiMouseEvent, View, type ViewProps } from "@gluxe/react";
import React, { useCallback, useMemo } from "react";

import { composeEventHandlers } from "./internal/compose";
import { createSafeContext } from "./internal/context";
import { useControllableState } from "./internal/controllable-state";
import { renderSlot, type Slot } from "./internal/slot";

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
  // `type`, `value`, `defaultValue`, `onValueChange` exist on both union members
  // so they destructure without a cast. `collapsible` is single-mode only, so it
  // lands in `rest` and is stripped separately (a tiny cast). Pulling every
  // control prop out keeps `viewProps` to genuine `<View>` props — leaking
  // `value`/`onValueChange`/etc. would forward them to the native element.
  const { type, disabled = false, children, value, defaultValue, onValueChange, ...rest } = props;
  const { collapsible: collapsibleProp = false, ...viewProps } = rest as {
    collapsible?: boolean;
  } & Omit<ViewProps, "children">;

  // --- single-mode controllable state (always called — hooks must be stable) ---
  const [singleValue, setSingleValue] = useControllableState<string>({
    prop: type === "single" ? (value as string | undefined) : undefined,
    defaultProp: type === "single" ? (defaultValue as string | undefined) : undefined,
    onChange: type === "single" ? (onValueChange as ((v: string) => void) | undefined) : undefined,
  });

  // --- multiple-mode controllable state (always called) ---------------------
  const [multipleValue = [], setMultipleValue] = useControllableState<string[]>({
    prop: type === "multiple" ? (value as unknown as string[] | undefined) : undefined,
    defaultProp:
      type === "multiple" ? (defaultValue as unknown as string[] | undefined) : undefined,
    onChange:
      type === "multiple"
        ? (onValueChange as unknown as ((v: string[]) => void) | undefined)
        : undefined,
  });

  // --- derive isOpen / toggle per mode -------------------------------------
  const collapsible = type === "single" ? collapsibleProp : false;

  const isOpen = useCallback(
    (itemValue: string): boolean => {
      if (type === "single") {
        return singleValue === itemValue;
      }
      return multipleValue.includes(itemValue);
    },
    [type, singleValue, multipleValue],
  );

  const toggle = useCallback(
    (itemValue: string): void => {
      if (type === "single") {
        if (singleValue === itemValue) {
          // Already open — only close if collapsible.
          if (collapsible) {
            // Use "" as the "none open" sentinel; isOpen checks strict equality
            // so an empty string never matches a real item value.
            setSingleValue("");
          }
        } else {
          setSingleValue(itemValue);
        }
      } else {
        if (multipleValue.includes(itemValue)) {
          setMultipleValue(multipleValue.filter((v) => v !== itemValue));
        } else {
          setMultipleValue([...multipleValue, itemValue]);
        }
      }
    },
    [type, singleValue, collapsible, setSingleValue, multipleValue, setMultipleValue],
  );

  const ctx = useMemo<AccordionContextValue>(
    () => ({ isOpen, toggle, disabled }),
    [isOpen, toggle, disabled],
  );

  return (
    <AccordionContextProvider value={ctx}>
      <View {...viewProps}>{children}</View>
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
 * closed on click; does nothing when the item or accordion is disabled.
 */
export function AccordionTrigger({
  children,
  onClick,
  ...viewProps
}: AccordionTriggerProps): React.ReactElement {
  const accordionCtx = useAccordionContext();
  const itemState = useAccordionItemContext();

  const handleClick = composeEventHandlers<GpuiMouseEvent>(onClick, () => {
    if (!itemState.disabled) {
      accordionCtx.toggle(itemState.value);
    }
  });

  return (
    <View {...viewProps} onClick={handleClick}>
      {renderSlot(children, itemState)}
    </View>
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
