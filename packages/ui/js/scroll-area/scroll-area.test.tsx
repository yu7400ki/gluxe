// @vitest-environment jsdom
//
// Tests for the ScrollArea headless component.
//
// Host elements (`View`, `__GluxeScrollbar`) are plain string types, so under
// react-dom + jsdom they render as custom DOM elements: a `<View>` becomes a
// `<view>` node and `<__GluxeScrollbar target={n} …>` becomes a custom element
// carrying the props as attributes. There is no Rust bridge here, so the
// Viewport's `ref` would normally receive a DOM node (no numeric `id`). To
// exercise the production id-flow deterministically, a test ref stamps a numeric
// `id` onto the node (simulating the runtime's `GpuiInstance`) before the
// component's own ref callback reads it (consumer refs run first in `mergeRefs`).
import { type GpuiInstance, Text } from "@gluxe/react";
import React, { act, useCallback } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { ScrollArea } from "./scroll-area";

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

/** The native scrollbar element (lowercased to `__gluxescrollbar` by jsdom). */
function scrollbarEl(): Element | null {
  return container.querySelector("__gluxescrollbar");
}

/**
 * A Viewport ref that stamps a fixed numeric `id` onto the host node before the
 * component reads it — simulating the runtime `GpuiInstance.id`. Returns a ref
 * callback to pass to `ScrollArea.Viewport`.
 */
function useStampedId(id: number): React.RefCallback<GpuiInstance> {
  return useCallback(
    (node) => {
      if (node) {
        Object.defineProperty(node, "id", { value: id, configurable: true });
      }
    },
    [id],
  );
}

describe("ScrollArea", () => {
  it("mounts without crashing and renders viewport content", () => {
    render(
      <ScrollArea>
        <ScrollArea.Viewport>
          <Text>content</Text>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar>
          <ScrollArea.Thumb />
        </ScrollArea.Scrollbar>
      </ScrollArea>,
    );
    expect(container.textContent).toContain("content");
  });

  it("the Scrollbar renders nothing until the viewport ref commits (no numeric id)", () => {
    // No stamped id → the viewport's ref carries no numeric `id`, so the
    // Scrollbar's target stays undefined and it renders null.
    render(
      <ScrollArea>
        <ScrollArea.Viewport>
          <Text>content</Text>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar>
          <ScrollArea.Thumb />
        </ScrollArea.Scrollbar>
      </ScrollArea>,
    );
    expect(scrollbarEl()).toBeNull();
  });

  it("the Scrollbar receives target = the viewport's host id once committed", () => {
    function Demo() {
      const ref = useStampedId(42);
      return (
        <ScrollArea>
          <ScrollArea.Viewport ref={ref}>
            <Text>content</Text>
          </ScrollArea.Viewport>
          <ScrollArea.Scrollbar>
            <ScrollArea.Thumb />
          </ScrollArea.Scrollbar>
        </ScrollArea>
      );
    }
    render(<Demo />);
    const bar = scrollbarEl();
    expect(bar).not.toBeNull();
    expect(bar?.getAttribute("target")).toBe("42");
  });

  it("orientation flows through to the native host element", () => {
    function Demo() {
      const ref = useStampedId(7);
      return (
        <ScrollArea>
          <ScrollArea.Viewport ref={ref} style={{ overflowX: "scroll", overflowY: "hidden" }}>
            <Text>content</Text>
          </ScrollArea.Viewport>
          <ScrollArea.Scrollbar orientation="horizontal">
            <ScrollArea.Thumb />
          </ScrollArea.Scrollbar>
        </ScrollArea>
      );
    }
    render(<Demo />);
    expect(scrollbarEl()?.getAttribute("orientation")).toBe("horizontal");
  });

  it("defaults orientation to vertical", () => {
    function Demo() {
      const ref = useStampedId(3);
      return (
        <ScrollArea>
          <ScrollArea.Viewport ref={ref}>
            <Text>content</Text>
          </ScrollArea.Viewport>
          <ScrollArea.Scrollbar>
            <ScrollArea.Thumb />
          </ScrollArea.Scrollbar>
        </ScrollArea>
      );
    }
    render(<Demo />);
    expect(scrollbarEl()?.getAttribute("orientation")).toBe("vertical");
  });

  it("maps the Thumb style to thumbColor / thumbRadius / minThumbLength / thumbInset", () => {
    function Demo() {
      const ref = useStampedId(9);
      return (
        <ScrollArea>
          <ScrollArea.Viewport ref={ref}>
            <Text>content</Text>
          </ScrollArea.Viewport>
          <ScrollArea.Scrollbar>
            <ScrollArea.Thumb
              style={{ backgroundColor: "#6ea8fe", borderRadius: 4, minHeight: 24, margin: 2 }}
            />
          </ScrollArea.Scrollbar>
        </ScrollArea>
      );
    }
    render(<Demo />);
    const bar = scrollbarEl();
    expect(bar?.getAttribute("thumbColor")).toBe("#6ea8fe");
    expect(bar?.getAttribute("thumbRadius")).toBe("4");
    expect(bar?.getAttribute("minThumbLength")).toBe("24");
    expect(bar?.getAttribute("thumbInset")).toBe("2");
  });

  it("maps Thumb _hover / _active background colours to thumbHoverColor / thumbActiveColor", () => {
    function Demo() {
      const ref = useStampedId(13);
      return (
        <ScrollArea>
          <ScrollArea.Viewport ref={ref}>
            <Text>content</Text>
          </ScrollArea.Viewport>
          <ScrollArea.Scrollbar>
            <ScrollArea.Thumb
              style={{
                backgroundColor: "#6ea8fe",
                _hover: { backgroundColor: "#8fbcff" },
                _active: { backgroundColor: "#3d6fd6" },
              }}
            />
          </ScrollArea.Scrollbar>
        </ScrollArea>
      );
    }
    render(<Demo />);
    const bar = scrollbarEl();
    expect(bar?.getAttribute("thumbColor")).toBe("#6ea8fe");
    expect(bar?.getAttribute("thumbHoverColor")).toBe("#8fbcff");
    expect(bar?.getAttribute("thumbActiveColor")).toBe("#3d6fd6");
  });

  it("maps minWidth (not minHeight) to minThumbLength for a horizontal scrollbar", () => {
    function Demo() {
      const ref = useStampedId(11);
      return (
        <ScrollArea>
          <ScrollArea.Viewport ref={ref}>
            <Text>content</Text>
          </ScrollArea.Viewport>
          <ScrollArea.Scrollbar orientation="horizontal">
            <ScrollArea.Thumb style={{ minWidth: 30, minHeight: 99 }} />
          </ScrollArea.Scrollbar>
        </ScrollArea>
      );
    }
    render(<Demo />);
    expect(scrollbarEl()?.getAttribute("minThumbLength")).toBe("30");
  });

  it("ScrollArea.Scrollbar throws when rendered outside <ScrollArea>", () => {
    expect(() =>
      render(
        <ScrollArea.Scrollbar>
          <ScrollArea.Thumb />
        </ScrollArea.Scrollbar>,
      ),
    ).toThrow("This component must be rendered inside <ScrollArea>.");
  });

  it("ScrollArea.Viewport throws when rendered outside <ScrollArea>", () => {
    expect(() =>
      render(
        <ScrollArea.Viewport>
          <Text>content</Text>
        </ScrollArea.Viewport>,
      ),
    ).toThrow("This component must be rendered inside <ScrollArea>.");
  });

  it("the Viewport is focusable by default (tabIndex 0) and the consumer can override it", () => {
    function Demo({ tabIndex }: { tabIndex?: number }) {
      return (
        <ScrollArea>
          <ScrollArea.Viewport tabIndex={tabIndex}>
            <Text>content</Text>
          </ScrollArea.Viewport>
        </ScrollArea>
      );
    }
    // Default: the Viewport joins the Tab order so keyboard users can scroll it.
    render(<Demo />);
    expect(container.querySelector("view[tabindex]")?.getAttribute("tabindex")).toBe("0");

    // Override: tabIndex={-1} keeps it scrollable but out of the Tab order.
    render(<Demo tabIndex={-1} />);
    expect(container.querySelector("view[tabindex]")?.getAttribute("tabindex")).toBe("-1");
  });
});
