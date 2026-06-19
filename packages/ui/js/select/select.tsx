import {
  type FloatingArea,
  type GpuiInstance,
  type GpuiKeyboardEvent,
  type GpuiMouseEvent,
  Text,
  View,
  type ViewProps,
} from "@gluxe/react";
import React, { useCallback, useEffect, useId, useMemo, useRef } from "react";

import { Button } from "../button/button";
import { composeEventHandlers } from "../internal/compose";
import { createSafeContext } from "../internal/context";
import { useControllableState } from "../internal/controllable-state";
import { mergeRefs } from "../internal/merge-refs";
import { type ListNavigation, useListItem, useListNavigation } from "../internal/roving-focus";
import { renderSlot, type Slot } from "../internal/slot";
import { Portal } from "../portal/portal";

/** State the Select root (and {@link SelectTrigger}) expose to render children. */
export interface SelectState {
  /** The currently selected value, or `undefined` when nothing is selected. */
  value: string | undefined;
  /** Whether the dropdown is open. */
  open: boolean;
  /** Whether the whole select is disabled. */
  disabled: boolean;
}

interface SelectContextValue {
  value: string | undefined;
  open: boolean;
  disabled: boolean;
  /** The generated `anchorName` shared by the trigger and the floating content. */
  anchorName: string;
  triggerRef: React.RefObject<GpuiInstance | null>;
  /** Open the dropdown (programmatically focuses an option on mount). */
  openMenu: () => void;
  /** Close the dropdown and return focus to the trigger. */
  closeAndFocusTrigger: () => void;
  /** Select a value, then close and return focus to the trigger. */
  select: (value: string) => void;
  /** Keyboard navigation + highlight channel for the open option list. */
  list: ListNavigation;
}

const [SelectContextProvider, useSelectContext] = createSafeContext<SelectContextValue>("Select");

/** State a {@link SelectItem} (and its children) exposes to render children. */
export interface SelectItemState {
  /** Whether this option is the selected value. */
  selected: boolean;
  /** Whether this option is highlighted (keyboard-focused) in the open list. */
  highlighted: boolean;
  /** Whether this option is disabled. */
  disabled: boolean;
  /** This option's value. */
  value: string;
}

interface SelectItemContextValue extends SelectItemState {}

const [SelectItemContextProvider, useSelectItemContext] =
  createSafeContext<SelectItemContextValue>("Select.Item");

export interface SelectProps extends Omit<ViewProps, "children"> {
  /** Controlled selected value. */
  value?: string;
  /** Initial selected value when uncontrolled. */
  defaultValue?: string;
  /** Called with the newly selected value when it changes. */
  onValueChange?: (value: string) => void;
  /** Controlled open state of the dropdown. */
  open?: boolean;
  /** Initial open state when uncontrolled. @default false */
  defaultOpen?: boolean;
  /** Called with the new open state whenever the dropdown opens or closes. */
  onOpenChange?: (open: boolean) => void;
  /**
   * Disables the trigger (it cannot be opened) and leaves it out of the Tab order.
   * Applies no visual style — read `disabled` to style it. @default false
   */
  disabled?: boolean;
  /** Wrap arrow navigation past the first / last option. @default true */
  loop?: boolean;
  children?: Slot<SelectState>;
}

/**
 * A single-select dropdown (combobox/listbox). Wraps a {@link SelectTrigger}
 * (the button that opens the list, doubling as the floating anchor) and a
 * {@link SelectContent} (the floating list of {@link SelectItem} options).
 *
 * Headless: no styles are applied. Read state via render-function children to
 * style by state, e.g. `<Select>{({ open }) => …}</Select>` or on each part.
 *
 * Supports controlled (`value` + `onValueChange`, `open` + `onOpenChange`) and
 * uncontrolled (`defaultValue` / `defaultOpen`) usage.
 *
 * **Keyboard & focus:** the trigger is the single Tab stop. Enter / Space (or a
 * click) opens the list; Down / Up also open it. While open, the selected (or
 * first) option is focused; Up / Down move between options (wrapping when `loop`,
 * skipping disabled ones), Home / End jump to the first / last, Enter / Space (or
 * a click) selects the focused option, and Escape closes — all returning focus to
 * the trigger. Clicking outside also closes. Style options with `_focusVisible`
 * or the `highlighted` render-prop state.
 */
export function Select({
  value: valueProp,
  defaultValue,
  onValueChange,
  open: openProp,
  defaultOpen,
  onOpenChange,
  disabled = false,
  loop = true,
  children,
  ...viewProps
}: SelectProps): React.ReactElement {
  const [value, setValue] = useControllableState<string>({
    prop: valueProp,
    defaultProp: defaultValue,
    onChange: onValueChange,
  });
  const [open = false, setOpen] = useControllableState<boolean>({
    prop: openProp,
    defaultProp: defaultOpen,
    onChange: onOpenChange,
  });

  // Unique anchor name per instance so the floating content positions against
  // this select's trigger (anchor names are a last-writer-wins global map).
  const reactId = useId();
  const anchorName = `gluxe-select-${reactId}`;

  const triggerRef = useRef<GpuiInstance | null>(null);
  const list = useListNavigation({ loop });

  // Reset the highlight once the list is closed so the next open starts clean.
  const { setHighlighted } = list;
  useEffect(() => {
    if (!open) setHighlighted(undefined);
  }, [open, setHighlighted]);

  const openMenu = useCallback(() => setOpen(true), [setOpen]);

  const closeAndFocusTrigger = useCallback(() => {
    setOpen(false);
    triggerRef.current?.focus();
  }, [setOpen]);

  const select = useCallback(
    (v: string) => {
      setValue(v);
      setOpen(false);
      triggerRef.current?.focus();
    },
    [setValue, setOpen],
  );

  const context = useMemo<SelectContextValue>(
    () => ({
      value,
      open,
      disabled,
      anchorName,
      triggerRef,
      openMenu,
      closeAndFocusTrigger,
      select,
      list,
    }),
    [value, open, disabled, anchorName, openMenu, closeAndFocusTrigger, select, list],
  );

  return (
    <SelectContextProvider value={context}>
      <View {...viewProps}>{renderSlot(children, { value, open, disabled })}</View>
    </SelectContextProvider>
  );
}
Select.displayName = "Select";

export interface SelectTriggerProps extends Omit<ViewProps, "children"> {
  children?: Slot<SelectState>;
}

/**
 * The button that opens the {@link SelectContent} dropdown and serves as its
 * floating anchor. A click (or Enter / Space while focused) toggles the list;
 * Down / Up open it. Inert while the select is disabled.
 *
 * Must be rendered inside `<Select>`.
 */
export function SelectTrigger({
  children,
  onClick,
  onKeyDown,
  ref,
  ...viewProps
}: SelectTriggerProps): React.ReactElement {
  const ctx = useSelectContext();

  const handleClick = composeEventHandlers<GpuiMouseEvent>(onClick, () => {
    if (ctx.disabled) return;
    if (ctx.open) ctx.closeAndFocusTrigger();
    else ctx.openMenu();
  });

  const handleKeyDown = composeEventHandlers<GpuiKeyboardEvent>(onKeyDown, (e) => {
    if (ctx.disabled || ctx.open) return;
    // Down / Up open the list (Enter / Space open via the runtime's click).
    if (e.key === "down" || e.key === "up") ctx.openMenu();
  });

  return (
    <Button
      {...viewProps}
      disabled={ctx.disabled}
      anchorName={ctx.anchorName}
      ref={mergeRefs(ref, ctx.triggerRef)}
      onClick={handleClick}
      onKeyDown={handleKeyDown}
    >
      {renderSlot(children, { value: ctx.value, open: ctx.open, disabled: ctx.disabled })}
    </Button>
  );
}
SelectTrigger.displayName = "Select.Trigger";

export interface SelectValueProps extends Omit<ViewProps, "children"> {
  /** Shown when nothing is selected. */
  placeholder?: string;
  /**
   * Render the display for the current value. Receives `{ value }` so you can
   * map a value to a human label. Defaults to rendering the raw value (or the
   * `placeholder` when none).
   */
  children?: Slot<{ value: string | undefined }>;
}

/**
 * Convenience display of the selected value, for placing inside the
 * {@link SelectTrigger}. With no `children` it renders the raw value (or
 * `placeholder`); pass a render function to map the value to a label.
 *
 * Must be rendered inside `<Select>`.
 */
export function SelectValue({
  placeholder,
  children,
  ...viewProps
}: SelectValueProps): React.ReactElement {
  const ctx = useSelectContext();
  const state = { value: ctx.value };

  return (
    <View {...viewProps}>
      {children !== undefined ? (
        renderSlot(children, state)
      ) : (
        <Text>{ctx.value ?? placeholder ?? ""}</Text>
      )}
    </View>
  );
}
SelectValue.displayName = "Select.Value";

export interface SelectContentProps extends Omit<ViewProps, "children"> {
  /**
   * Placement of the list relative to the trigger.
   * @default "bottom start"
   */
  area?: FloatingArea;
  /**
   * Gap from the trigger along the `area` side, in px (or a `"px"`/`"rem"` string).
   * @default 4
   */
  offset?: number | `${number}px` | `${number}rem`;
  children?: Slot<SelectState>;
}

/**
 * The floating list of options, anchored to the {@link SelectTrigger}. Mounts
 * only while the dropdown is open (it is unmounted, not hidden). Lifts above
 * in-flow content and clipping via the runtime's floating overlay, so it can be
 * authored here without portals.
 *
 * Wrap any number of {@link SelectItem} options. Clicking outside the list closes
 * it (via a transparent full-window dismiss layer).
 *
 * Must be rendered inside `<Select>`.
 */
export function SelectContent(props: SelectContentProps): React.ReactElement | null {
  const ctx = useSelectContext();
  if (!ctx.open) return null;
  return <SelectContentImpl {...props} />;
}
SelectContent.displayName = "Select.Content";

// Mounted only while open, so its mount effect runs on every open — used to focus
// the selected (or first) option as the list appears.
function SelectContentImpl({
  area = "bottom start",
  offset = 4,
  children,
  ...viewProps
}: SelectContentProps): React.ReactElement {
  const ctx = useSelectContext();

  // On open, highlight & focus the selected option (or the first enabled one) so
  // keyboard navigation has a starting point. Option effects run before this
  // (children mount first), so the registry is already populated.
  useEffect(() => {
    ctx.list.focusInitial(ctx.value);
    // Run once when the list mounts (i.e. opens).
    // oxlint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <>
      {/* Transparent full-window dismiss layer: a click anywhere outside the
          (floating, on-top) list lands here and closes. Portaled so it covers the
          window regardless of where the Select sits in the tree. */}
      <Portal>
        <View
          style={{ position: "absolute", inset: 0 }}
          onClick={() => ctx.closeAndFocusTrigger()}
        />
      </Portal>
      <View {...viewProps} floating={{ anchor: ctx.anchorName, area, offset }}>
        {renderSlot(children, { value: ctx.value, open: ctx.open, disabled: ctx.disabled })}
      </View>
    </>
  );
}

export interface SelectItemProps extends Omit<ViewProps, "children"> {
  /** This option's value; must be unique within the select. */
  value: string;
  /** Disables this option (cannot be selected or focused). @default false */
  disabled?: boolean;
  /**
   * Text matched by keyboard type-ahead (case-insensitive prefix). Defaults to
   * `value`; set it when the visible label differs from `value` (e.g.
   * `value="us"` with `textValue="United States"`) so type-ahead matches the
   * label the user sees rather than the raw value.
   */
  textValue?: string;
  children?: Slot<SelectItemState>;
}

/**
 * A single selectable option inside {@link SelectContent}. Clicking it (or
 * pressing Enter / Space while it is the focused option) selects its `value`,
 * closes the list, and returns focus to the trigger.
 *
 * Must be rendered inside `<Select.Content>`.
 */
export function SelectItem({
  value: itemValue,
  disabled: itemDisabled = false,
  textValue,
  children,
  onClick,
  onKeyDown,
  onFocus,
  ref,
  // Options are focused programmatically, never via Tab; an explicit tabIndex is ignored.
  tabIndex: _tabIndex,
  ...viewProps
}: SelectItemProps): React.ReactElement {
  const ctx = useSelectContext();
  const disabled = ctx.disabled || itemDisabled;
  const selected = ctx.value === itemValue;
  const highlighted = ctx.list.highlighted === itemValue;

  const item = useListItem(ctx.list, itemValue, disabled, textValue);

  const handleClick = composeEventHandlers<GpuiMouseEvent>(onClick, () => {
    if (!disabled) ctx.select(itemValue);
  });

  // Escape closes the list; arrow / Home / End move the highlight. Selection
  // comes from the runtime's click (Enter / Space activate the focused option).
  const handleKeyDown = composeEventHandlers<GpuiKeyboardEvent>(onKeyDown, (e) => {
    if (e.key === "escape") {
      ctx.closeAndFocusTrigger();
      return;
    }
    item.onKeyDown(e);
  });

  const handleFocus = composeEventHandlers(onFocus, item.onFocus);

  const itemCtx = useMemo<SelectItemContextValue>(
    () => ({ selected, highlighted, disabled, value: itemValue }),
    [selected, highlighted, disabled, itemValue],
  );

  return (
    <SelectItemContextProvider value={itemCtx}>
      <View
        {...viewProps}
        ref={mergeRefs(ref, item.ref)}
        tabIndex={-1}
        onClick={disabled ? undefined : handleClick}
        onKeyDown={handleKeyDown}
        onFocus={handleFocus}
      >
        {renderSlot(children, itemCtx)}
      </View>
    </SelectItemContextProvider>
  );
}
SelectItem.displayName = "Select.Item";

export interface SelectItemIndicatorProps extends Omit<ViewProps, "children"> {
  children?: Slot<SelectItemState>;
}

/**
 * Visual selection indicator for a {@link SelectItem}. Renders nothing unless the
 * parent option is selected; mount a custom mark (e.g. a checkmark) as children.
 *
 * Must be rendered inside `<Select.Item>`.
 */
export function SelectItemIndicator({
  children,
  ...viewProps
}: SelectItemIndicatorProps): React.ReactElement | null {
  const itemState = useSelectItemContext();
  if (!itemState.selected) return null;
  return <View {...viewProps}>{renderSlot(children, itemState)}</View>;
}
SelectItemIndicator.displayName = "Select.ItemIndicator";

Select.Trigger = SelectTrigger;
Select.Value = SelectValue;
Select.Content = SelectContent;
Select.Item = SelectItem;
Select.ItemIndicator = SelectItemIndicator;
