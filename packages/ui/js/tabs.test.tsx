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

function click(text: string): void {
  const leaf = [...container.querySelectorAll("*")].find(
    (el) => el.children.length === 0 && el.textContent === text,
  );
  if (!leaf) throw new Error(`No element with text "${text}" in:\n${container.innerHTML}`);
  act(() => {
    leaf.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
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
});
