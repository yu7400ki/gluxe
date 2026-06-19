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

function click(text: string): void {
  const leaf = [...container.querySelectorAll("*")].find(
    (el) => el.children.length === 0 && el.textContent === text,
  );
  if (!leaf) throw new Error(`No element with text "${text}" in:\n${container.innerHTML}`);
  act(() => {
    leaf.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
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
});
