// @vitest-environment jsdom
//
// Tests for the Select headless component.
import { Text, View } from "@gluxe/react";
import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Select } from "./select";

// The dismiss layer renders through <Portal>, which uses the runtime's
// `createPortal` (a separate reconciler that needs the native bridge). Under
// jsdom there is no bridge, so stub Portal to render its children inline.
vi.mock("../portal/portal", () => ({
  Portal: ({ children }: { children?: React.ReactNode }) => children,
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

/** Dispatch a GPUI-named key (e.g. "down", "escape") on the leaf labelled `text`. */
function keydown(text: string, key: string): void {
  act(() => {
    leafWith(text).dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
  });
}

const FRUITS = ["apple", "banana", "cherry"] as const;

/** A select with three fruit options; `b` (banana) optionally disabled. */
function FruitSelect({
  onValueChange,
  onOpenChange,
  defaultValue,
  defaultOpen,
  open,
  disabled,
  disabledValue,
}: {
  onValueChange?: (v: string) => void;
  onOpenChange?: (o: boolean) => void;
  defaultValue?: string;
  defaultOpen?: boolean;
  open?: boolean;
  disabled?: boolean;
  disabledValue?: string;
}) {
  return (
    <Select
      onValueChange={onValueChange}
      onOpenChange={onOpenChange}
      defaultValue={defaultValue}
      defaultOpen={defaultOpen}
      open={open}
      disabled={disabled}
    >
      <Select.Trigger>
        <Select.Value placeholder="pick" />
      </Select.Trigger>
      <Select.Content>
        {FRUITS.map((v) => (
          <Select.Item key={v} value={v} disabled={v === disabledValue}>
            {({ highlighted, selected }) => (
              <View>
                <Text>{v}</Text>
                {highlighted ? <Text>{`${v}-hl`}</Text> : null}
                {selected ? <Text>{`${v}-sel`}</Text> : null}
              </View>
            )}
          </Select.Item>
        ))}
      </Select.Content>
    </Select>
  );
}

describe("Select", () => {
  it("the option list is not mounted while closed", () => {
    render(<FruitSelect />);
    expect(container.textContent).toContain("pick");
    expect(container.textContent).not.toContain("banana");
  });

  it("clicking the trigger opens the list", () => {
    render(<FruitSelect />);
    click("pick");
    expect(container.textContent).toContain("apple");
    expect(container.textContent).toContain("banana");
  });

  it("onOpenChange fires when opening", () => {
    const onOpenChange = vi.fn();
    render(<FruitSelect onOpenChange={onOpenChange} />);
    click("pick");
    expect(onOpenChange).toHaveBeenCalledWith(true);
  });

  it("clicking an option selects it, fires onValueChange, and closes the list", () => {
    const onValueChange = vi.fn();
    render(<FruitSelect onValueChange={onValueChange} />);
    click("pick");
    click("banana");
    expect(onValueChange).toHaveBeenCalledOnce();
    expect(onValueChange).toHaveBeenCalledWith("banana");
    // List closed: only the trigger's value display remains.
    expect(container.textContent).not.toContain("apple");
    expect(container.textContent).toContain("banana");
  });

  it("Select.Value shows the selected value after selection", () => {
    render(<FruitSelect />);
    click("pick");
    click("cherry");
    expect(container.textContent).toContain("cherry");
    expect(container.textContent).not.toContain("pick");
  });

  it("defaultValue marks the matching option selected (Indicator via render-prop)", () => {
    render(<FruitSelect defaultValue="banana" defaultOpen />);
    expect(container.textContent).toContain("banana-sel");
    expect(container.textContent).not.toContain("apple-sel");
  });

  it("a disabled select does not open on click", () => {
    const onOpenChange = vi.fn();
    render(<FruitSelect disabled onOpenChange={onOpenChange} />);
    click("pick");
    expect(onOpenChange).not.toHaveBeenCalled();
    expect(container.textContent).not.toContain("banana");
  });

  it("a disabled option cannot be selected", () => {
    const onValueChange = vi.fn();
    render(<FruitSelect defaultOpen disabledValue="banana" onValueChange={onValueChange} />);
    click("banana");
    expect(onValueChange).not.toHaveBeenCalled();
  });

  it("controlled open renders the list", () => {
    render(<FruitSelect open />);
    expect(container.textContent).toContain("cherry");
  });

  describe("keyboard navigation", () => {
    it("opening highlights the selected option (or the first when none)", () => {
      render(<FruitSelect defaultOpen />);
      expect(container.textContent).toContain("apple-hl");
    });

    it("opening highlights the selected option", () => {
      render(<FruitSelect defaultOpen defaultValue="cherry" />);
      expect(container.textContent).toContain("cherry-hl");
      expect(container.textContent).not.toContain("apple-hl");
    });

    it("ArrowDown moves the highlight to the next option", () => {
      render(<FruitSelect defaultOpen />);
      keydown("apple", "down");
      expect(container.textContent).toContain("banana-hl");
      expect(container.textContent).not.toContain("apple-hl");
    });

    it("ArrowUp wraps to the last option", () => {
      render(<FruitSelect defaultOpen />);
      keydown("apple", "up");
      expect(container.textContent).toContain("cherry-hl");
    });

    it("Home / End jump to the first / last option", () => {
      render(<FruitSelect defaultOpen defaultValue="banana" />);
      // Key off the unique "-hl" leaf: with a selected value the trigger also
      // renders a "banana" leaf, so the bare value would be ambiguous.
      keydown("banana-hl", "end");
      expect(container.textContent).toContain("cherry-hl");
      keydown("cherry-hl", "home");
      expect(container.textContent).toContain("apple-hl");
    });

    it("arrow navigation skips a disabled option", () => {
      render(<FruitSelect defaultOpen disabledValue="banana" />);
      keydown("apple", "down");
      expect(container.textContent).toContain("cherry-hl");
      expect(container.textContent).not.toContain("banana-hl");
    });

    it("Escape closes the list", () => {
      const onOpenChange = vi.fn();
      render(<FruitSelect defaultOpen onOpenChange={onOpenChange} />);
      keydown("apple", "escape");
      expect(onOpenChange).toHaveBeenCalledWith(false);
      expect(container.textContent).not.toContain("banana");
    });

    it("does not change the value while navigating (selection is explicit)", () => {
      const onValueChange = vi.fn();
      render(<FruitSelect defaultOpen onValueChange={onValueChange} />);
      keydown("apple", "down");
      keydown("banana", "down");
      expect(onValueChange).not.toHaveBeenCalled();
    });

    it("type-ahead jumps the highlight to the option matching the typed char", () => {
      render(<FruitSelect defaultOpen />); // apple highlighted
      keydown("apple", "c");
      expect(container.textContent).toContain("cherry-hl");
      expect(container.textContent).not.toContain("apple-hl");
    });

    it("the type-ahead buffer resets after the idle window", () => {
      // Fake only the type-ahead timers so React's scheduler keeps real ones.
      vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
      try {
        render(<FruitSelect defaultOpen />); // apple highlighted
        keydown("apple", "b"); // buffer "b" → banana
        expect(container.textContent).toContain("banana-hl");

        // Let the idle window elapse so the buffer clears.
        act(() => {
          vi.advanceTimersByTime(600);
        });

        // A fresh "c" starts a new search (had the buffer survived, "bc" would
        // match nothing and the highlight would stay on banana).
        keydown("banana", "c");
        expect(container.textContent).toContain("cherry-hl");
        expect(container.textContent).not.toContain("banana-hl");
      } finally {
        vi.useRealTimers();
      }
    });
  });
});
