import {
  type GpuiInstance,
  type GpuiKeyboardEvent,
  type GpuiMouseEvent,
  View,
  type ViewProps,
} from "@gluxe/react";
import React, { useCallback, useMemo, useRef } from "react";

import { Button } from "../button/button";
import { composeEventHandlers } from "../internal/compose";
import { createSafeContext } from "../internal/context";
import { useControllableState } from "../internal/controllable-state";
import { FocusScope } from "../internal/focus-scope";
import { mergeRefs } from "../internal/merge-refs";
import { renderSlot, type Slot } from "../internal/slot";
import { Portal } from "../portal/portal";

/** State the Dialog parts expose to their render-function children. */
export interface DialogState {
  /** Whether the dialog is open. */
  open: boolean;
}

interface DialogContextValue {
  open: boolean;
  triggerRef: React.RefObject<GpuiInstance | null>;
  /** Open the dialog. */
  openDialog: () => void;
  /** Close the dialog. */
  close: () => void;
}

const [DialogContextProvider, useDialogContext] = createSafeContext<DialogContextValue>("Dialog");

export interface DialogProps {
  /** Controlled open state. */
  open?: boolean;
  /** Initial open state when uncontrolled. @default false */
  defaultOpen?: boolean;
  /** Called with the new open state whenever the dialog opens or closes. */
  onOpenChange?: (open: boolean) => void;
  children?: Slot<DialogState>;
}

/**
 * A modal dialog: a {@link DialogTrigger}, an optional {@link DialogOverlay}
 * (backdrop), and a {@link DialogPositioner} holding the {@link DialogContent}
 * panel and any {@link DialogClose} buttons. The overlay, positioner, and content
 * mount only while open and render through a portal.
 *
 * Headless: no styles are applied, and the root renders no element of its own.
 * Supports controlled (`open` + `onOpenChange`) and uncontrolled (`defaultOpen`)
 * usage; read state via render-function children to style by state.
 *
 * Opening focuses the panel; closing returns focus to the previously-focused
 * element. Tab is trapped inside the panel — it wraps at the edges, so it can't
 * reach elements behind the dialog. Escape and a backdrop click close it.
 *
 * @example
 * <Dialog>
 *   <Dialog.Trigger><Text>Open</Text></Dialog.Trigger>
 *   <Dialog.Overlay style={{ backgroundColor: "rgba(0,0,0,0.5)" }} />
 *   <Dialog.Positioner>
 *     <Dialog.Content style={{ width: 320, padding: 20, backgroundColor: "#181b22" }}>
 *       <Text>Hello</Text>
 *       <Dialog.Close><Text>Close</Text></Dialog.Close>
 *     </Dialog.Content>
 *   </Dialog.Positioner>
 * </Dialog>
 */
export function Dialog({
  open: openProp,
  defaultOpen,
  onOpenChange,
  children,
}: DialogProps): React.ReactElement {
  const [open = false, setOpen] = useControllableState<boolean>({
    prop: openProp,
    defaultProp: defaultOpen,
    onChange: onOpenChange,
  });

  const triggerRef = useRef<GpuiInstance | null>(null);

  const openDialog = useCallback(() => setOpen(true), [setOpen]);
  const close = useCallback(() => setOpen(false), [setOpen]);

  const context = useMemo<DialogContextValue>(
    () => ({ open, triggerRef, openDialog, close }),
    [open, openDialog, close],
  );

  return (
    <DialogContextProvider value={context}>{renderSlot(children, { open })}</DialogContextProvider>
  );
}
Dialog.displayName = "Dialog";

export interface DialogTriggerProps extends Omit<ViewProps, "children"> {
  children?: Slot<DialogState>;
}

/**
 * The button that opens the dialog. A click (or Enter / Space while focused)
 * opens it. Closing returns focus here.
 *
 * Must be rendered inside `<Dialog>`.
 */
export function DialogTrigger({
  children,
  onClick,
  ref,
  ...viewProps
}: DialogTriggerProps): React.ReactElement {
  const ctx = useDialogContext();

  return (
    <Button
      {...viewProps}
      ref={mergeRefs(ref, ctx.triggerRef)}
      onClick={composeEventHandlers<GpuiMouseEvent>(onClick, ctx.openDialog)}
    >
      {renderSlot(children, { open: ctx.open })}
    </Button>
  );
}
DialogTrigger.displayName = "Dialog.Trigger";

export interface DialogOverlayProps extends Omit<ViewProps, "children"> {
  /** Close the dialog when the backdrop is clicked. @default true */
  closeOnClick?: boolean;
  children?: Slot<DialogState>;
}

/**
 * The modal backdrop: a full-window portaled layer behind the panel that blocks
 * pointer interaction with the page and dismisses on click (toggle with
 * `closeOnClick`). Clicks on the panel don't reach it (see {@link DialogContent}).
 *
 * Apply a `backgroundColor` to dim the page. Positioned `absolute, inset: 0`
 * (override via `style`). Omit it for a non-modal dialog.
 *
 * Must be rendered inside `<Dialog>`.
 */
export function DialogOverlay(props: DialogOverlayProps): React.ReactElement | null {
  const ctx = useDialogContext();
  if (!ctx.open) return null;
  return <DialogOverlayImpl {...props} />;
}
DialogOverlay.displayName = "Dialog.Overlay";

function DialogOverlayImpl({
  closeOnClick = true,
  children,
  style,
  onClick,
  ...viewProps
}: DialogOverlayProps): React.ReactElement {
  const ctx = useDialogContext();

  const handleClick = composeEventHandlers<GpuiMouseEvent>(onClick, () => {
    if (closeOnClick) ctx.close();
  });

  return (
    <Portal>
      <View
        {...viewProps}
        style={{ position: "absolute", inset: 0, ...style }}
        onClick={handleClick}
      >
        {renderSlot(children, { open: ctx.open })}
      </View>
    </Portal>
  );
}

export interface DialogPositionerProps extends Omit<ViewProps, "children"> {
  children?: Slot<DialogState>;
}

/**
 * The full-window portaled layer that places the {@link DialogContent} panel. A
 * flex container positioned `absolute, inset: 0`, centring its child by default
 * (override `alignItems` / `justifyContent` via `style`).
 *
 * Pure layout: pointer-transparent (`occlude={false}`), so clicks around the
 * panel fall through to the {@link DialogOverlay} behind it. Wrap a single
 * {@link DialogContent}.
 *
 * Must be rendered inside `<Dialog>`.
 */
export function DialogPositioner(props: DialogPositionerProps): React.ReactElement | null {
  const ctx = useDialogContext();
  if (!ctx.open) return null;
  return <DialogPositionerImpl {...props} />;
}
DialogPositioner.displayName = "Dialog.Positioner";

function DialogPositionerImpl({
  children,
  style,
  ...viewProps
}: DialogPositionerProps): React.ReactElement {
  const ctx = useDialogContext();
  return (
    <Portal>
      <View
        {...viewProps}
        // Transparent to the mouse so outside clicks reach the overlay behind.
        occlude={false}
        style={{
          position: "absolute",
          inset: 0,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          ...style,
        }}
      >
        {renderSlot(children, { open: ctx.open })}
      </View>
    </Portal>
  );
}

export interface DialogContentProps extends Omit<ViewProps, "children"> {
  /** Pressing Escape while focus is within the dialog closes it. @default true */
  closeOnEscape?: boolean;
  children?: Slot<DialogState>;
}

/**
 * The dialog panel, rendered inside a {@link DialogPositioner} as a flex child
 * (sizes to its content). It `occlude`s so clicks on it don't reach the backdrop
 * and close the dialog (override with `occlude={false}`).
 *
 * On open, focus moves to the first control inside it; Tab is trapped within the
 * panel; on close focus returns to the previously-focused element. Escape closes
 * it (toggle with `closeOnEscape`).
 *
 * Must be rendered inside a `<Dialog.Positioner>`.
 */
export function DialogContent(props: DialogContentProps): React.ReactElement | null {
  const ctx = useDialogContext();
  if (!ctx.open) return null;
  return <DialogContentImpl {...props} />;
}
DialogContent.displayName = "Dialog.Content";

// Mounted only while open, so FocusScope's mount effect runs on every open.
function DialogContentImpl({
  closeOnEscape = true,
  children,
  onKeyDown,
  ref,
  tabIndex,
  occlude = true,
  ...viewProps
}: DialogContentProps): React.ReactElement {
  const ctx = useDialogContext();
  const panelRef = useRef<GpuiInstance | null>(null);

  const handleKeyDown = composeEventHandlers<GpuiKeyboardEvent>(onKeyDown, (e) => {
    if (closeOnEscape && e.key === "escape") ctx.close();
  });

  // FocusScope focuses the first control inside on open, traps Tab within the
  // panel, and restores focus to the prior element on close.
  return (
    <FocusScope containerRef={panelRef}>
      <View
        {...viewProps}
        ref={mergeRefs(ref, panelRef)}
        occlude={occlude}
        tabIndex={tabIndex ?? -1}
        onKeyDown={handleKeyDown}
      >
        {renderSlot(children, { open: ctx.open })}
      </View>
    </FocusScope>
  );
}

export interface DialogCloseProps extends Omit<ViewProps, "children"> {
  children?: Slot<DialogState>;
}

/**
 * A button that closes the dialog on click (or Enter / Space while focused).
 * Place it anywhere inside {@link DialogContent}.
 *
 * Must be rendered inside `<Dialog>`.
 */
export function DialogClose({
  children,
  onClick,
  ...viewProps
}: DialogCloseProps): React.ReactElement {
  const ctx = useDialogContext();

  return (
    <Button {...viewProps} onClick={composeEventHandlers<GpuiMouseEvent>(onClick, ctx.close)}>
      {renderSlot(children, { open: ctx.open })}
    </Button>
  );
}
DialogClose.displayName = "Dialog.Close";

Dialog.Trigger = DialogTrigger;
Dialog.Overlay = DialogOverlay;
Dialog.Positioner = DialogPositioner;
Dialog.Content = DialogContent;
Dialog.Close = DialogClose;
