// @vitest-environment jsdom
//
// Tests for the RadioGroup headless component.
import { Text, View } from "@gluxe/react";
import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { RadioGroup } from "./radio-group";

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

/** Dispatch a GPUI-named key (e.g. "down", "home") on the leaf labelled `text`. */
function keydown(text: string, key: string): void {
  act(() => {
    leafWith(text).dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
  });
}

/** The `tabindex` of the focusable item element whose label leaf is `text`. */
function tabIndexFor(text: string): string | null {
  let el: Element | null = leafWith(text);
  while (el && !el.hasAttribute("tabindex")) el = el.parentElement;
  return el ? el.getAttribute("tabindex") : null;
}

/** A simple radio group with two items (apple / banana). */
function TwoItemGroup({
  onValueChange,
  disabled,
  defaultValue,
}: {
  onValueChange?: (v: string) => void;
  disabled?: boolean;
  defaultValue?: string;
}) {
  return (
    <RadioGroup onValueChange={onValueChange} disabled={disabled} defaultValue={defaultValue}>
      <RadioGroup.Item value="apple">
        <Text>apple</Text>
        <RadioGroup.Indicator>
          <Text>apple-selected</Text>
        </RadioGroup.Indicator>
      </RadioGroup.Item>
      <RadioGroup.Item value="banana">
        <Text>banana</Text>
        <RadioGroup.Indicator>
          <Text>banana-selected</Text>
        </RadioGroup.Indicator>
      </RadioGroup.Item>
    </RadioGroup>
  );
}

describe("RadioGroup", () => {
  it("no item is selected initially when defaultValue is omitted", () => {
    render(<TwoItemGroup />);
    expect(container.textContent).not.toContain("apple-selected");
    expect(container.textContent).not.toContain("banana-selected");
  });

  it("clicking an item selects it (shows Indicator)", () => {
    render(<TwoItemGroup />);
    click("apple");
    expect(container.textContent).toContain("apple-selected");
  });

  it("selecting another item deselects the first (Indicator moves)", () => {
    render(<TwoItemGroup />);
    click("apple");
    expect(container.textContent).toContain("apple-selected");
    expect(container.textContent).not.toContain("banana-selected");

    click("banana");
    expect(container.textContent).toContain("banana-selected");
    expect(container.textContent).not.toContain("apple-selected");
  });

  it("onValueChange fires with the selected item's value", () => {
    const onChange = vi.fn();
    render(<TwoItemGroup onValueChange={onChange} />);
    click("apple");
    expect(onChange).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledWith("apple");
  });

  it("defaultValue pre-selects the matching item", () => {
    render(<TwoItemGroup defaultValue="banana" />);
    expect(container.textContent).toContain("banana-selected");
    expect(container.textContent).not.toContain("apple-selected");
  });

  it("group disabled blocks all items", () => {
    const onChange = vi.fn();
    render(<TwoItemGroup onValueChange={onChange} disabled />);
    click("apple");
    expect(onChange).not.toHaveBeenCalled();
    expect(container.textContent).not.toContain("apple-selected");
  });

  it("per-item disabled blocks only that item", () => {
    const onChange = vi.fn();
    render(
      <RadioGroup onValueChange={onChange}>
        <RadioGroup.Item value="apple" disabled>
          <Text>apple</Text>
          <RadioGroup.Indicator>
            <Text>apple-selected</Text>
          </RadioGroup.Indicator>
        </RadioGroup.Item>
        <RadioGroup.Item value="banana">
          <Text>banana</Text>
          <RadioGroup.Indicator>
            <Text>banana-selected</Text>
          </RadioGroup.Indicator>
        </RadioGroup.Item>
      </RadioGroup>,
    );
    click("apple");
    expect(onChange).not.toHaveBeenCalled();
    expect(container.textContent).not.toContain("apple-selected");

    click("banana");
    expect(onChange).toHaveBeenCalledWith("banana");
    expect(container.textContent).toContain("banana-selected");
  });

  it("item render-prop exposes checked and disabled", () => {
    render(
      <RadioGroup>
        <RadioGroup.Item value="apple">
          {({ checked, disabled }) => (
            <View>
              <Text>{checked ? "yes" : "no"}</Text>
              <Text>{disabled ? "dis" : "ena"}</Text>
            </View>
          )}
        </RadioGroup.Item>
      </RadioGroup>,
    );
    expect(container.textContent).toContain("no");
    expect(container.textContent).toContain("ena");
    click("no");
    expect(container.textContent).toContain("yes");
  });

  describe("roving focus / keyboard", () => {
    /** Three radio items (a / b / c), `b` optionally disabled. */
    function ThreeItemGroup({ disabledValue }: { disabledValue?: string }) {
      return (
        <RadioGroup>
          {["a", "b", "c"].map((v) => (
            <RadioGroup.Item key={v} value={v} disabled={v === disabledValue}>
              <Text>{v}</Text>
              <RadioGroup.Indicator>
                <Text>{`${v}-on`}</Text>
              </RadioGroup.Indicator>
            </RadioGroup.Item>
          ))}
        </RadioGroup>
      );
    }

    it("the first enabled item is the only Tab stop when none is selected", () => {
      render(<TwoItemGroup />);
      expect(tabIndexFor("apple")).toBe("0");
      expect(tabIndexFor("banana")).toBe("-1");
    });

    it("the selected item owns the Tab stop", () => {
      render(<TwoItemGroup defaultValue="banana" />);
      expect(tabIndexFor("banana")).toBe("0");
      expect(tabIndexFor("apple")).toBe("-1");
    });

    it("a disabled selected item does not become the Tab stop", () => {
      // "a" is selected but disabled → the stop goes to the first enabled item.
      render(
        <RadioGroup value="a">
          <RadioGroup.Item value="a" disabled>
            <Text>a</Text>
          </RadioGroup.Item>
          <RadioGroup.Item value="b">
            <Text>b</Text>
          </RadioGroup.Item>
        </RadioGroup>,
      );
      expect(tabIndexFor("a")).toBe("-1");
      expect(tabIndexFor("b")).toBe("0");
    });

    it("a disabled first item hands the Tab stop to the next enabled item", () => {
      render(<ThreeItemGroup disabledValue="a" />);
      expect(tabIndexFor("a")).toBe("-1");
      expect(tabIndexFor("b")).toBe("0");
    });

    it("ArrowDown selects the next item (selection follows focus)", () => {
      const onChange = vi.fn();
      render(<TwoItemGroup onValueChange={onChange} />);
      keydown("apple", "down");
      expect(onChange).toHaveBeenCalledWith("banana");
      expect(container.textContent).toContain("banana-selected");
      expect(tabIndexFor("banana")).toBe("0");
    });

    it("ArrowUp wraps to the last item", () => {
      render(<TwoItemGroup />);
      keydown("apple", "up");
      expect(container.textContent).toContain("banana-selected");
    });

    it("responds to all four arrows (orientation 'both')", () => {
      render(<TwoItemGroup />);
      keydown("apple", "right");
      expect(container.textContent).toContain("banana-selected");
    });

    it("Home/End jump to the first/last enabled item", () => {
      render(<ThreeItemGroup />);
      keydown("a", "end");
      expect(container.textContent).toContain("c-on");
      keydown("c", "home");
      expect(container.textContent).toContain("a-on");
    });

    it("arrow navigation skips a disabled item", () => {
      render(<ThreeItemGroup disabledValue="b" />);
      keydown("a", "down");
      expect(container.textContent).toContain("c-on");
      expect(container.textContent).not.toContain("b-on");
    });

    // Selection comes from the runtime's synthesized click, not a key-down (double-fire guard).
    it("does not select on a raw Space/Enter key-down", () => {
      const onChange = vi.fn();
      render(<TwoItemGroup onValueChange={onChange} />);
      keydown("apple", "space");
      keydown("apple", "enter");
      expect(onChange).not.toHaveBeenCalled();
      expect(container.textContent).not.toContain("apple-selected");
    });

    it("a fully disabled group exposes no Tab stop", () => {
      render(<TwoItemGroup disabled />);
      expect(tabIndexFor("apple")).toBe("-1");
      expect(tabIndexFor("banana")).toBe("-1");
    });
  });
});
