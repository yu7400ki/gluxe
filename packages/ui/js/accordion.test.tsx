// @vitest-environment jsdom
//
// Tests for the Accordion headless component (single and multiple modes).
import { Text, View } from "@gluxe/react";
import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Accordion } from "./accordion";

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

/** Dispatch a GPUI-named key (e.g. "space", "enter") on the leaf with `text`. */
function keydown(text: string, key: string): void {
  const leaf = [...container.querySelectorAll("*")].find(
    (el) => el.children.length === 0 && el.textContent === text,
  );
  if (!leaf) throw new Error(`No element with text "${text}" in:\n${container.innerHTML}`);
  act(() => {
    leaf.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
  });
}

/** Two-item accordion used across single-mode tests. */
function SingleAccordion({
  collapsible,
  onValueChange,
  defaultValue,
}: {
  collapsible?: boolean;
  onValueChange?: (v: string) => void;
  defaultValue?: string;
}) {
  return (
    <Accordion
      type="single"
      collapsible={collapsible}
      onValueChange={onValueChange}
      defaultValue={defaultValue}
    >
      <Accordion.Item value="a">
        <Accordion.Trigger>
          <Text>header-a</Text>
        </Accordion.Trigger>
        <Accordion.Content>
          <Text>content-a</Text>
        </Accordion.Content>
      </Accordion.Item>
      <Accordion.Item value="b">
        <Accordion.Trigger>
          <Text>header-b</Text>
        </Accordion.Trigger>
        <Accordion.Content>
          <Text>content-b</Text>
        </Accordion.Content>
      </Accordion.Item>
    </Accordion>
  );
}

/** Two-item accordion in multiple mode. */
function MultiAccordion({
  onValueChange,
  defaultValue,
}: {
  onValueChange?: (v: string[]) => void;
  defaultValue?: string[];
}) {
  return (
    <Accordion type="multiple" onValueChange={onValueChange} defaultValue={defaultValue}>
      <Accordion.Item value="a">
        <Accordion.Trigger>
          <Text>header-a</Text>
        </Accordion.Trigger>
        <Accordion.Content>
          <Text>content-a</Text>
        </Accordion.Content>
      </Accordion.Item>
      <Accordion.Item value="b">
        <Accordion.Trigger>
          <Text>header-b</Text>
        </Accordion.Trigger>
        <Accordion.Content>
          <Text>content-b</Text>
        </Accordion.Content>
      </Accordion.Item>
    </Accordion>
  );
}

describe("Accordion — type=single", () => {
  it("all items are closed by default", () => {
    render(<SingleAccordion />);
    expect(container.textContent).not.toContain("content-a");
    expect(container.textContent).not.toContain("content-b");
  });

  it("clicking a trigger opens its content", () => {
    render(<SingleAccordion />);
    click("header-a");
    expect(container.textContent).toContain("content-a");
  });

  it("opening one item closes the other (only one open at a time)", () => {
    render(<SingleAccordion />);
    click("header-a");
    expect(container.textContent).toContain("content-a");
    click("header-b");
    expect(container.textContent).toContain("content-b");
    expect(container.textContent).not.toContain("content-a");
  });

  it("without collapsible, clicking the open item keeps it open", () => {
    render(<SingleAccordion collapsible={false} />);
    click("header-a");
    expect(container.textContent).toContain("content-a");
    click("header-a");
    // still open — collapsible is false (default)
    expect(container.textContent).toContain("content-a");
  });

  it("with collapsible, clicking the open item closes it", () => {
    render(<SingleAccordion collapsible={true} />);
    click("header-a");
    expect(container.textContent).toContain("content-a");
    click("header-a");
    expect(container.textContent).not.toContain("content-a");
  });

  it("onValueChange fires with the opened item value", () => {
    const onChange = vi.fn();
    render(<SingleAccordion onValueChange={onChange} />);
    click("header-a");
    expect(onChange).toHaveBeenCalledWith("a");
  });

  it("defaultValue pre-opens an item", () => {
    render(<SingleAccordion defaultValue="b" />);
    expect(container.textContent).toContain("content-b");
    expect(container.textContent).not.toContain("content-a");
  });

  it("AccordionContent is absent when item is closed", () => {
    render(<SingleAccordion />);
    expect(container.textContent).not.toContain("content-a");
    expect(container.textContent).not.toContain("content-b");
  });

  it("item render-prop exposes open state", () => {
    render(
      <Accordion type="single">
        <Accordion.Item value="x">
          {({ open }) => (
            <View>
              <Accordion.Trigger>
                <Text>header-x</Text>
              </Accordion.Trigger>
              <Text>{open ? "is-open" : "is-closed"}</Text>
            </View>
          )}
        </Accordion.Item>
      </Accordion>,
    );
    expect(container.textContent).toContain("is-closed");
    click("header-x");
    expect(container.textContent).toContain("is-open");
  });
});

describe("Accordion — type=multiple", () => {
  it("multiple items can be open simultaneously", () => {
    render(<MultiAccordion />);
    click("header-a");
    click("header-b");
    expect(container.textContent).toContain("content-a");
    expect(container.textContent).toContain("content-b");
  });

  it("clicking an open item closes it independently", () => {
    render(<MultiAccordion defaultValue={["a", "b"]} />);
    expect(container.textContent).toContain("content-a");
    expect(container.textContent).toContain("content-b");
    click("header-a");
    expect(container.textContent).not.toContain("content-a");
    expect(container.textContent).toContain("content-b");
  });

  it("onValueChange fires with the new array of open values", () => {
    const onChange = vi.fn();
    render(<MultiAccordion onValueChange={onChange} />);
    click("header-a");
    expect(onChange).toHaveBeenCalledWith(["a"]);
    click("header-b");
    expect(onChange).toHaveBeenCalledWith(["a", "b"]);
  });

  it("defaultValue pre-opens multiple items", () => {
    render(<MultiAccordion defaultValue={["a", "b"]} />);
    expect(container.textContent).toContain("content-a");
    expect(container.textContent).toContain("content-b");
  });
});

describe("Accordion — keyboard / focus", () => {
  it("every trigger is focusable (each header has tabIndex 0, no roving)", () => {
    render(<SingleAccordion />);
    const tabbable = [...container.querySelectorAll("[tabindex]")];
    expect(tabbable.map((el) => el.getAttribute("tabindex"))).toEqual(["0", "0"]);
  });

  it("a disabled item's trigger is removed from the Tab order", () => {
    render(
      <Accordion type="single">
        <Accordion.Item value="a" disabled>
          <Accordion.Trigger>
            <Text>header-a</Text>
          </Accordion.Trigger>
          <Accordion.Content>
            <Text>content-a</Text>
          </Accordion.Content>
        </Accordion.Item>
        <Accordion.Item value="b">
          <Accordion.Trigger>
            <Text>header-b</Text>
          </Accordion.Trigger>
          <Accordion.Content>
            <Text>content-b</Text>
          </Accordion.Content>
        </Accordion.Item>
      </Accordion>,
    );
    // Only the enabled item's trigger is tabbable.
    expect(container.querySelectorAll("[tabindex]").length).toBe(1);
  });

  // Activation comes from the runtime's synthesized click, not a key-down (double-fire guard).
  it("does not toggle on a raw Space/Enter key-down (avoids double activation)", () => {
    render(<SingleAccordion />);
    keydown("header-a", "space");
    keydown("header-a", "enter");
    expect(container.textContent).not.toContain("content-a");
  });
});
