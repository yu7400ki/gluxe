// @vitest-environment jsdom
//
// Tests for the Toggle headless component.
// Renders through react-dom: gluxe host elements become unknown DOM tags.
import { Text, View } from "@gluxe/react";
import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Toggle } from "./toggle";

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

describe("Toggle", () => {
  it("starts unpressed by default", () => {
    render(
      <Toggle>
        {({ pressed }) => (
          <View>
            <Text>{pressed ? "pressed" : "unpressed"}</Text>
          </View>
        )}
      </Toggle>,
    );
    expect(container.textContent).toContain("unpressed");
  });

  it("clicking flips pressed from false to true", () => {
    render(
      <Toggle>
        {({ pressed }) => (
          <View>
            <Text>{pressed ? "pressed" : "unpressed"}</Text>
          </View>
        )}
      </Toggle>,
    );
    expect(container.textContent).toContain("unpressed");
    click("unpressed");
    expect(container.textContent).toContain("pressed");
  });

  it("clicking again flips pressed back to false", () => {
    render(
      <Toggle defaultPressed={true}>
        {({ pressed }) => (
          <View>
            <Text>{pressed ? "pressed" : "unpressed"}</Text>
          </View>
        )}
      </Toggle>,
    );
    expect(container.textContent).toContain("pressed");
    click("pressed");
    expect(container.textContent).toContain("unpressed");
  });

  it("onPressedChange fires with the next value", () => {
    const onChange = vi.fn();
    render(
      <Toggle onPressedChange={onChange}>
        <Text>btn</Text>
      </Toggle>,
    );
    click("btn");
    expect(onChange).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledWith(true);

    click("btn");
    expect(onChange).toHaveBeenCalledTimes(2);
    expect(onChange).toHaveBeenLastCalledWith(false);
  });

  it("disabled blocks the state change", () => {
    render(
      <Toggle disabled>
        {({ pressed }) => (
          <View>
            <Text>{pressed ? "pressed" : "unpressed"}</Text>
          </View>
        )}
      </Toggle>,
    );
    click("unpressed");
    expect(container.textContent).toContain("unpressed");
  });

  it("disabled prevents onPressedChange from firing", () => {
    const onChange = vi.fn();
    render(
      <Toggle disabled onPressedChange={onChange}>
        <Text>btn</Text>
      </Toggle>,
    );
    click("btn");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("controlled mode: state follows the prop", () => {
    render(
      <Toggle pressed={true}>
        {({ pressed }) => (
          <View>
            <Text>{pressed ? "pressed" : "unpressed"}</Text>
          </View>
        )}
      </Toggle>,
    );
    expect(container.textContent).toContain("pressed");
  });
});
