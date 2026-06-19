// @vitest-environment jsdom
//
// Tests for the Checkbox headless component (tri-state: true | false | "indeterminate").
import { Text, View } from "@gluxe/react";
import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Checkbox } from "./checkbox";

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

describe("Checkbox", () => {
  it("Indicator is absent when unchecked", () => {
    render(
      <Checkbox defaultChecked={false}>
        <View>
          <Text>check</Text>
          <Checkbox.Indicator>
            <Text>indicator</Text>
          </Checkbox.Indicator>
        </View>
      </Checkbox>,
    );
    expect(container.textContent).not.toContain("indicator");
  });

  it("Indicator is present when checked", () => {
    render(
      <Checkbox defaultChecked={true}>
        <View>
          <Text>check</Text>
          <Checkbox.Indicator>
            <Text>indicator</Text>
          </Checkbox.Indicator>
        </View>
      </Checkbox>,
    );
    expect(container.textContent).toContain("indicator");
  });

  it("clicking toggles from unchecked to checked", () => {
    render(
      <Checkbox defaultChecked={false}>
        <View>
          <Text>check</Text>
          <Checkbox.Indicator>
            <Text>indicator</Text>
          </Checkbox.Indicator>
        </View>
      </Checkbox>,
    );
    expect(container.textContent).not.toContain("indicator");
    click("check");
    expect(container.textContent).toContain("indicator");
  });

  it("clicking toggles from checked to unchecked", () => {
    render(
      <Checkbox defaultChecked={true}>
        <View>
          <Text>check</Text>
          <Checkbox.Indicator>
            <Text>indicator</Text>
          </Checkbox.Indicator>
        </View>
      </Checkbox>,
    );
    expect(container.textContent).toContain("indicator");
    click("check");
    expect(container.textContent).not.toContain("indicator");
  });

  it("onCheckedChange fires with true when toggling from unchecked", () => {
    const onChange = vi.fn();
    render(
      <Checkbox defaultChecked={false} onCheckedChange={onChange}>
        <Text>check</Text>
      </Checkbox>,
    );
    click("check");
    expect(onChange).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("onCheckedChange fires with false when toggling from checked", () => {
    const onChange = vi.fn();
    render(
      <Checkbox defaultChecked={true} onCheckedChange={onChange}>
        <Text>check</Text>
      </Checkbox>,
    );
    click("check");
    expect(onChange).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it("indeterminate checkbox shows the Indicator", () => {
    render(
      <Checkbox defaultChecked="indeterminate">
        <View>
          <Text>check</Text>
          <Checkbox.Indicator>
            <Text>indicator</Text>
          </Checkbox.Indicator>
        </View>
      </Checkbox>,
    );
    expect(container.textContent).toContain("indicator");
  });

  it("clicking an indeterminate checkbox calls onCheckedChange(true)", () => {
    const onChange = vi.fn();
    render(
      <Checkbox defaultChecked="indeterminate" onCheckedChange={onChange}>
        <Text>check</Text>
      </Checkbox>,
    );
    click("check");
    expect(onChange).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("indeterminate Indicator uses render-prop to show the checked state", () => {
    render(
      <Checkbox defaultChecked="indeterminate">
        <View>
          <Text>check</Text>
          <Checkbox.Indicator>
            {({ checked }) => <Text>{checked === "indeterminate" ? "dash" : "tick"}</Text>}
          </Checkbox.Indicator>
        </View>
      </Checkbox>,
    );
    expect(container.textContent).toContain("dash");
  });

  it("disabled blocks state change", () => {
    render(
      <Checkbox defaultChecked={false} disabled>
        <View>
          <Text>check</Text>
          <Checkbox.Indicator>
            <Text>indicator</Text>
          </Checkbox.Indicator>
        </View>
      </Checkbox>,
    );
    click("check");
    expect(container.textContent).not.toContain("indicator");
  });

  it("disabled prevents onCheckedChange from firing", () => {
    const onChange = vi.fn();
    render(
      <Checkbox defaultChecked={false} disabled onCheckedChange={onChange}>
        <Text>check</Text>
      </Checkbox>,
    );
    click("check");
    expect(onChange).not.toHaveBeenCalled();
  });
});
