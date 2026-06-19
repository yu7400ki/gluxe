// @vitest-environment jsdom
//
// Tests for the Tabs headless component.
import { Text, View } from "@gluxe/react";
import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Tabs } from "./tabs";

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

/** Dispatch a GPUI-named key (e.g. "right", "home") on the leaf with `text`. */
function keydown(text: string, key: string): void {
  act(() => {
    leafWith(text).dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
  });
}

/** The `tabindex` of the focusable trigger element wrapping `text`. */
function tabIndexOf(text: string): string | null {
  const el = [...container.querySelectorAll("[tabindex]")].find((e) => e.textContent === text);
  return el ? el.getAttribute("tabindex") : null;
}

/** Two-tab panel used across most tests. */
function TwoTabs({
  defaultValue,
  onValueChange,
}: {
  defaultValue?: string;
  onValueChange?: (v: string) => void;
}) {
  return (
    <Tabs defaultValue={defaultValue} onValueChange={onValueChange}>
      <Tabs.List>
        <Tabs.Trigger value="one">
          <Text>tab-one</Text>
        </Tabs.Trigger>
        <Tabs.Trigger value="two">
          <Text>tab-two</Text>
        </Tabs.Trigger>
      </Tabs.List>
      <Tabs.Content value="one">
        <Text>panel-one</Text>
      </Tabs.Content>
      <Tabs.Content value="two">
        <Text>panel-two</Text>
      </Tabs.Content>
    </Tabs>
  );
}

describe("Tabs", () => {
  it("only the active tab's Content is mounted", () => {
    render(<TwoTabs defaultValue="one" />);
    expect(container.textContent).toContain("panel-one");
    expect(container.textContent).not.toContain("panel-two");
  });

  it("clicking a trigger switches the visible content", () => {
    render(<TwoTabs defaultValue="one" />);
    click("tab-two");
    expect(container.textContent).toContain("panel-two");
    expect(container.textContent).not.toContain("panel-one");
  });

  it("the previous tab's content is unmounted on switch", () => {
    render(<TwoTabs defaultValue="one" />);
    expect(container.textContent).toContain("panel-one");
    click("tab-two");
    expect(container.textContent).not.toContain("panel-one");
  });

  it("no content is shown when no defaultValue is provided and value is undefined", () => {
    render(
      <Tabs>
        <Tabs.List>
          <Tabs.Trigger value="a">
            <Text>tab-a</Text>
          </Tabs.Trigger>
        </Tabs.List>
        <Tabs.Content value="a">
          <Text>panel-a</Text>
        </Tabs.Content>
      </Tabs>,
    );
    expect(container.textContent).not.toContain("panel-a");
  });

  it("onValueChange fires with the newly selected tab's value", () => {
    const onChange = vi.fn();
    render(<TwoTabs defaultValue="one" onValueChange={onChange} />);
    click("tab-two");
    expect(onChange).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledWith("two");
  });

  it("trigger render-prop exposes selected state", () => {
    render(
      <Tabs defaultValue="one">
        <Tabs.List>
          <Tabs.Trigger value="one">
            {({ selected }) => <Text>{selected ? "one-active" : "one-inactive"}</Text>}
          </Tabs.Trigger>
          <Tabs.Trigger value="two">
            {({ selected }) => <Text>{selected ? "two-active" : "two-inactive"}</Text>}
          </Tabs.Trigger>
        </Tabs.List>
      </Tabs>,
    );
    expect(container.textContent).toContain("one-active");
    expect(container.textContent).toContain("two-inactive");
    click("one-active");
    // clicking active tab again — still active
    expect(container.textContent).toContain("one-active");
  });

  it("a disabled trigger does not switch tabs", () => {
    render(
      <Tabs defaultValue="one">
        <Tabs.List>
          <Tabs.Trigger value="one">
            <Text>tab-one</Text>
          </Tabs.Trigger>
          <Tabs.Trigger value="two" disabled>
            <Text>tab-two</Text>
          </Tabs.Trigger>
        </Tabs.List>
        <Tabs.Content value="one">
          <Text>panel-one</Text>
        </Tabs.Content>
        <Tabs.Content value="two">
          <Text>panel-two</Text>
        </Tabs.Content>
      </Tabs>,
    );
    click("tab-two");
    expect(container.textContent).toContain("panel-one");
    expect(container.textContent).not.toContain("panel-two");
  });

  it("a disabled trigger does not fire onValueChange", () => {
    const onChange = vi.fn();
    render(
      <Tabs defaultValue="one" onValueChange={onChange}>
        <Tabs.List>
          <Tabs.Trigger value="one">
            <Text>tab-one</Text>
          </Tabs.Trigger>
          <Tabs.Trigger value="two" disabled>
            <Text>tab-two</Text>
          </Tabs.Trigger>
        </Tabs.List>
      </Tabs>,
    );
    click("tab-two");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("Tabs.Content passes selected=true to render-prop children while mounted", () => {
    render(
      <Tabs defaultValue="one">
        <Tabs.List>
          <Tabs.Trigger value="one">
            <Text>tab-one</Text>
          </Tabs.Trigger>
        </Tabs.List>
        <Tabs.Content value="one">
          {({ selected }) => <Text>{selected ? "selected" : "not-selected"}</Text>}
        </Tabs.Content>
      </Tabs>,
    );
    expect(container.textContent).toContain("selected");
  });

  it("Tabs.List renders its children without filtering", () => {
    render(
      <Tabs defaultValue="a">
        <Tabs.List>
          <View>
            <Tabs.Trigger value="a">
              <Text>tab-a</Text>
            </Tabs.Trigger>
          </View>
        </Tabs.List>
        <Tabs.Content value="a">
          <Text>panel-a</Text>
        </Tabs.Content>
      </Tabs>,
    );
    expect(container.textContent).toContain("panel-a");
  });

  describe("roving focus / keyboard", () => {
    it("the selected trigger is the only Tab stop", () => {
      render(<TwoTabs defaultValue="one" />);
      expect(tabIndexOf("tab-one")).toBe("0");
      expect(tabIndexOf("tab-two")).toBe("-1");
    });

    it("the first trigger is the Tab stop when nothing is selected", () => {
      render(<TwoTabs />);
      expect(tabIndexOf("tab-one")).toBe("0");
      expect(tabIndexOf("tab-two")).toBe("-1");
    });

    it("a disabled selected trigger does not become the Tab stop", () => {
      // "one" is selected but disabled → the stop goes to the first enabled trigger.
      render(
        <Tabs value="one">
          <Tabs.List>
            <Tabs.Trigger value="one" disabled>
              <Text>tab-one</Text>
            </Tabs.Trigger>
            <Tabs.Trigger value="two">
              <Text>tab-two</Text>
            </Tabs.Trigger>
          </Tabs.List>
        </Tabs>,
      );
      expect(tabIndexOf("tab-one")).toBe("-1");
      expect(tabIndexOf("tab-two")).toBe("0");
    });

    it("ArrowRight selects the next tab (automatic activation)", () => {
      render(<TwoTabs defaultValue="one" />);
      keydown("tab-one", "right");
      expect(container.textContent).toContain("panel-two");
      expect(tabIndexOf("tab-two")).toBe("0");
      expect(tabIndexOf("tab-one")).toBe("-1");
    });

    it("ArrowLeft wraps to the last tab by default", () => {
      render(<TwoTabs defaultValue="one" />);
      keydown("tab-one", "left");
      expect(container.textContent).toContain("panel-two");
    });

    it("does not wrap when loop is false", () => {
      const onChange = vi.fn();
      render(
        <Tabs defaultValue="one" loop={false} onValueChange={onChange}>
          <Tabs.List>
            <Tabs.Trigger value="one">
              <Text>tab-one</Text>
            </Tabs.Trigger>
            <Tabs.Trigger value="two">
              <Text>tab-two</Text>
            </Tabs.Trigger>
          </Tabs.List>
        </Tabs>,
      );
      keydown("tab-one", "left");
      expect(onChange).not.toHaveBeenCalled();
    });

    it("vertical orientation navigates with Up/Down, ignores Left/Right", () => {
      render(
        <Tabs defaultValue="one" orientation="vertical">
          <Tabs.List>
            <Tabs.Trigger value="one">
              <Text>tab-one</Text>
            </Tabs.Trigger>
            <Tabs.Trigger value="two">
              <Text>tab-two</Text>
            </Tabs.Trigger>
          </Tabs.List>
          <Tabs.Content value="one">
            <Text>panel-one</Text>
          </Tabs.Content>
          <Tabs.Content value="two">
            <Text>panel-two</Text>
          </Tabs.Content>
        </Tabs>,
      );
      keydown("tab-one", "right");
      expect(container.textContent).toContain("panel-one");
      keydown("tab-one", "down");
      expect(container.textContent).toContain("panel-two");
    });

    it("Home/End jump to the first/last tab", () => {
      const tabs = (
        <Tabs defaultValue="two">
          <Tabs.List>
            <Tabs.Trigger value="one">
              <Text>tab-one</Text>
            </Tabs.Trigger>
            <Tabs.Trigger value="two">
              <Text>tab-two</Text>
            </Tabs.Trigger>
            <Tabs.Trigger value="three">
              <Text>tab-three</Text>
            </Tabs.Trigger>
          </Tabs.List>
        </Tabs>
      );
      render(tabs);
      keydown("tab-two", "home");
      expect(tabIndexOf("tab-one")).toBe("0");
      keydown("tab-one", "end");
      expect(tabIndexOf("tab-three")).toBe("0");
    });

    it("arrow navigation skips a disabled trigger", () => {
      render(
        <Tabs defaultValue="one">
          <Tabs.List>
            <Tabs.Trigger value="one">
              <Text>tab-one</Text>
            </Tabs.Trigger>
            <Tabs.Trigger value="two" disabled>
              <Text>tab-two</Text>
            </Tabs.Trigger>
            <Tabs.Trigger value="three">
              <Text>tab-three</Text>
            </Tabs.Trigger>
          </Tabs.List>
          <Tabs.Content value="three">
            <Text>panel-three</Text>
          </Tabs.Content>
        </Tabs>,
      );
      keydown("tab-one", "right");
      expect(container.textContent).toContain("panel-three");
    });

    it("manual activation moves focus without selecting", () => {
      render(
        <Tabs defaultValue="one" activationMode="manual">
          <Tabs.List>
            <Tabs.Trigger value="one">
              <Text>tab-one</Text>
            </Tabs.Trigger>
            <Tabs.Trigger value="two">
              <Text>tab-two</Text>
            </Tabs.Trigger>
          </Tabs.List>
          <Tabs.Content value="one">
            <Text>panel-one</Text>
          </Tabs.Content>
          <Tabs.Content value="two">
            <Text>panel-two</Text>
          </Tabs.Content>
        </Tabs>,
      );
      keydown("tab-one", "right");
      // Focus moved (tab stop), selection unchanged; the click (not a key-down) selects.
      expect(tabIndexOf("tab-two")).toBe("0");
      expect(container.textContent).toContain("panel-one");
      click("tab-two");
      expect(container.textContent).toContain("panel-two");
    });

    // Selection comes from the runtime's synthesized click, not a key-down (double-fire guard).
    it("does not select on a raw Space/Enter key-down", () => {
      const onChange = vi.fn();
      render(
        <Tabs defaultValue="one" onValueChange={onChange}>
          <Tabs.List>
            <Tabs.Trigger value="one">
              <Text>tab-one</Text>
            </Tabs.Trigger>
            <Tabs.Trigger value="two">
              <Text>tab-two</Text>
            </Tabs.Trigger>
          </Tabs.List>
        </Tabs>,
      );
      keydown("tab-one", "space");
      keydown("tab-one", "enter");
      expect(onChange).not.toHaveBeenCalled();
    });
  });

  it("the active Content panel is focusable by default (tabIndex 0) and can be overridden", () => {
    function Panels({ tabIndex }: { tabIndex?: number }) {
      return (
        <Tabs defaultValue="one">
          <Tabs.List>
            <Tabs.Trigger value="one">
              <Text>tab-one</Text>
            </Tabs.Trigger>
          </Tabs.List>
          <Tabs.Content value="one" tabIndex={tabIndex}>
            <Text>panel-one</Text>
          </Tabs.Content>
        </Tabs>
      );
    }
    // Default: the panel joins the Tab order so keyboard users can reach its body.
    render(<Panels />);
    expect(tabIndexOf("panel-one")).toBe("0");

    // Override: tabIndex={-1} keeps the panel out of the Tab order.
    render(<Panels tabIndex={-1} />);
    expect(tabIndexOf("panel-one")).toBe("-1");
  });
});
