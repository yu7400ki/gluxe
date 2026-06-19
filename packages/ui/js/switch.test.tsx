// @vitest-environment jsdom
//
// Tests for the Switch headless component.
import { Text, View } from "@gluxe/react";
import React, { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Switch } from "./switch";

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

describe("Switch", () => {
  it("starts unchecked by default", () => {
    render(
      <Switch>
        {({ checked }) => (
          <View>
            <Text>{checked ? "on" : "off"}</Text>
          </View>
        )}
      </Switch>,
    );
    expect(container.textContent).toContain("off");
  });

  it("clicking toggles checked from false to true", () => {
    render(
      <Switch>
        {({ checked }) => (
          <View>
            <Text>{checked ? "on" : "off"}</Text>
          </View>
        )}
      </Switch>,
    );
    click("off");
    expect(container.textContent).toContain("on");
  });

  it("clicking toggles checked back to false", () => {
    render(
      <Switch defaultChecked={true}>
        {({ checked }) => (
          <View>
            <Text>{checked ? "on" : "off"}</Text>
          </View>
        )}
      </Switch>,
    );
    click("on");
    expect(container.textContent).toContain("off");
  });

  it("Switch.Thumb is always present regardless of checked state", () => {
    render(
      <Switch>
        {({ checked }) => (
          <View>
            <Text>{checked ? "on" : "off"}</Text>
            <Switch.Thumb>
              <Text>thumb</Text>
            </Switch.Thumb>
          </View>
        )}
      </Switch>,
    );
    expect(container.textContent).toContain("thumb");
    click("off");
    expect(container.textContent).toContain("thumb");
  });

  it("Switch.Thumb receives checked state via render-prop", () => {
    render(
      <Switch>
        <View>
          <Text>switch</Text>
          <Switch.Thumb>
            {({ checked }) => <Text>{checked ? "thumb-on" : "thumb-off"}</Text>}
          </Switch.Thumb>
        </View>
      </Switch>,
    );
    expect(container.textContent).toContain("thumb-off");
    click("switch");
    expect(container.textContent).toContain("thumb-on");
  });

  it("controlled mode: does NOT change without a prop update", () => {
    render(
      <Switch checked={false}>
        {({ checked }) => (
          <View>
            <Text>{checked ? "on" : "off"}</Text>
          </View>
        )}
      </Switch>,
    );
    click("off");
    // prop is still false — state must not change
    expect(container.textContent).toContain("off");
  });

  it("controlled mode: onCheckedChange fires but state stays put without re-render", () => {
    const onChange = vi.fn();
    render(
      <Switch checked={false} onCheckedChange={onChange}>
        {({ checked }) => (
          <View>
            <Text>{checked ? "on" : "off"}</Text>
          </View>
        )}
      </Switch>,
    );
    click("off");
    expect(onChange).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledWith(true);
    // state is still off because no prop update happened
    expect(container.textContent).toContain("off");
  });

  it("controlled mode: state tracks prop when parent updates it", () => {
    function Controlled() {
      const [checked, setChecked] = useState(false);
      return (
        <Switch checked={checked} onCheckedChange={setChecked}>
          {({ checked: c }) => (
            <View>
              <Text>{c ? "on" : "off"}</Text>
            </View>
          )}
        </Switch>
      );
    }
    render(<Controlled />);
    expect(container.textContent).toContain("off");
    click("off");
    expect(container.textContent).toContain("on");
  });

  it("disabled blocks state change", () => {
    render(
      <Switch disabled>
        {({ checked }) => (
          <View>
            <Text>{checked ? "on" : "off"}</Text>
          </View>
        )}
      </Switch>,
    );
    click("off");
    expect(container.textContent).toContain("off");
  });

  it("disabled prevents onCheckedChange from firing", () => {
    const onChange = vi.fn();
    render(
      <Switch disabled onCheckedChange={onChange}>
        <Text>switch</Text>
      </Switch>,
    );
    click("switch");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("is focusable by default (tabIndex 0)", () => {
    render(
      <Switch>
        <Text>switch</Text>
      </Switch>,
    );
    expect(container.querySelector("[tabindex]")?.getAttribute("tabindex")).toBe("0");
  });

  it("a disabled switch is removed from the Tab order", () => {
    render(
      <Switch disabled>
        <Text>switch</Text>
      </Switch>,
    );
    expect(container.querySelector("[tabindex]")).toBeNull();
  });

  // Activation comes from the runtime's synthesized click, not a key-down (double-fire guard).
  it("does not toggle on a raw Space/Enter key-down (avoids double activation)", () => {
    const onChange = vi.fn();
    render(
      <Switch onCheckedChange={onChange}>
        <Text>switch</Text>
      </Switch>,
    );
    keydown("switch", "space");
    keydown("switch", "enter");
    expect(onChange).not.toHaveBeenCalled();
  });
});
