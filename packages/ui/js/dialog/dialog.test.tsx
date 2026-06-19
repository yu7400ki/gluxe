// @vitest-environment jsdom
//
// Tests for the Dialog headless component.
import { Text } from "@gluxe/react";
import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Dialog } from "./dialog";

// Overlay and Content render through <Portal>, which uses the runtime's
// `createPortal` (a separate reconciler that needs the native bridge). Under
// jsdom there is no bridge, so stub Portal to render its children inline.
vi.mock("../portal/portal", () => ({
  Portal: ({ children }: { children?: React.ReactNode }) => children,
}));

// The focus API talks to the native `__bridge` / `__invoke` globals, absent
// under jsdom. Stub it so the content's focus-restore effect is a no-op.
vi.mock("@gluxe/react/focus", () => ({
  getActiveElement: () => null,
  focusElement: () => Promise.resolve(),
  focusFirstElement: () => Promise.resolve(),
  getFocusableElements: () => [],
  pushTabScope: () => {},
  popTabScope: () => {},
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let container: HTMLElement;
let root: Root;

beforeEach(() => {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

function render(element: React.ReactElement): void {
  act(() => root.render(element));
}

function leafWith(text: string): Element {
  const leaf = [...container.querySelectorAll("*")].find(
    (el) => el.children.length === 0 && el.textContent === text,
  );
  if (!leaf) throw new Error(`No element with text "${text}" in:\n${container.innerHTML}`);
  return leaf;
}

function click(text: string): void {
  act(() => {
    leafWith(text).dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
}

/** Dispatch a GPUI-named key (e.g. "escape") on the leaf labelled `text`. */
function keydown(text: string, key: string): void {
  act(() => {
    leafWith(text).dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
  });
}

function BasicDialog({
  open,
  defaultOpen,
  onOpenChange,
  closeOnEscape,
  closeOnClick,
}: {
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (o: boolean) => void;
  closeOnEscape?: boolean;
  closeOnClick?: boolean;
}) {
  return (
    <Dialog open={open} defaultOpen={defaultOpen} onOpenChange={onOpenChange}>
      <Dialog.Trigger>
        <Text>open</Text>
      </Dialog.Trigger>
      <Dialog.Overlay closeOnClick={closeOnClick}>
        {/* A child of the backdrop: stands in for clicking the empty backdrop
            area. Clicking it bubbles to the overlay's handler (natively, outside
            clicks reach the overlay by passing through the transparent positioner
            — that occlusion behaviour is absent under jsdom). */}
        <Text>backdrop</Text>
      </Dialog.Overlay>
      <Dialog.Positioner>
        <Dialog.Content closeOnEscape={closeOnEscape}>
          <Text>body</Text>
          <Dialog.Close>
            <Text>close</Text>
          </Dialog.Close>
        </Dialog.Content>
      </Dialog.Positioner>
    </Dialog>
  );
}

describe("Dialog", () => {
  it("the positioner and content are not mounted while closed", () => {
    render(<BasicDialog />);
    expect(container.textContent).toContain("open");
    expect(container.textContent).not.toContain("body");
    expect(container.textContent).not.toContain("backdrop");
  });

  it("clicking the trigger opens the dialog", () => {
    render(<BasicDialog />);
    click("open");
    expect(container.textContent).toContain("body");
    expect(container.textContent).toContain("backdrop");
  });

  it("onOpenChange fires when opening", () => {
    const onOpenChange = vi.fn();
    render(<BasicDialog onOpenChange={onOpenChange} />);
    click("open");
    expect(onOpenChange).toHaveBeenCalledWith(true);
  });

  it("clicking Close closes the dialog and fires onOpenChange", () => {
    const onOpenChange = vi.fn();
    render(<BasicDialog defaultOpen onOpenChange={onOpenChange} />);
    click("close");
    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(container.textContent).not.toContain("body");
  });

  it("clicking the backdrop (outside the panel) closes the dialog", () => {
    const onOpenChange = vi.fn();
    render(<BasicDialog defaultOpen onOpenChange={onOpenChange} />);
    click("backdrop");
    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(container.textContent).not.toContain("body");
  });

  it("closeOnClick={false} keeps the dialog open on a backdrop click", () => {
    const onOpenChange = vi.fn();
    render(<BasicDialog defaultOpen closeOnClick={false} onOpenChange={onOpenChange} />);
    click("backdrop");
    expect(onOpenChange).not.toHaveBeenCalled();
    expect(container.textContent).toContain("body");
  });

  it("Escape closes the dialog", () => {
    const onOpenChange = vi.fn();
    render(<BasicDialog defaultOpen onOpenChange={onOpenChange} />);
    keydown("body", "escape");
    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(container.textContent).not.toContain("body");
  });

  it("closeOnEscape={false} keeps the dialog open on Escape", () => {
    const onOpenChange = vi.fn();
    render(<BasicDialog defaultOpen closeOnEscape={false} onOpenChange={onOpenChange} />);
    keydown("body", "escape");
    expect(onOpenChange).not.toHaveBeenCalled();
    expect(container.textContent).toContain("body");
  });

  it("controlled open renders the content", () => {
    render(<BasicDialog open />);
    expect(container.textContent).toContain("body");
  });

  it("controlled open ignores internal toggles (stays open until prop changes)", () => {
    const onOpenChange = vi.fn();
    render(<BasicDialog open onOpenChange={onOpenChange} />);
    click("close");
    // Still open: the controller did not change the `open` prop.
    expect(container.textContent).toContain("body");
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("exposes open state to render-function children", () => {
    render(
      <Dialog defaultOpen>
        {({ open }) => (
          <>
            <Dialog.Trigger>
              <Text>{open ? "is-open" : "is-closed"}</Text>
            </Dialog.Trigger>
            <Dialog.Positioner>
              <Dialog.Content>
                <Text>body</Text>
              </Dialog.Content>
            </Dialog.Positioner>
          </>
        )}
      </Dialog>,
    );
    expect(container.textContent).toContain("is-open");
  });
});
