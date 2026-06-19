// @vitest-environment jsdom
//
// Tests for the Button headless component.
// Renders through react-dom: gluxe host elements become unknown DOM tags.
import { Text } from "@gluxe/react";
import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Button } from "./button";

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

/** Dispatch a GPUI-named key (e.g. "space", "enter") on the leaf with `text`. */
function keydown(text: string, key: string): void {
  act(() => {
    leafWith(text).dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
  });
}

describe("Button", () => {
  it("fires onClick when clicked", () => {
    const onClick = vi.fn();
    render(
      <Button onClick={onClick}>
        <Text>press</Text>
      </Button>,
    );
    click("press");
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("is focusable by default (tabIndex 0)", () => {
    render(
      <Button>
        <Text>press</Text>
      </Button>,
    );
    expect(container.querySelector("[tabindex]")?.getAttribute("tabindex")).toBe("0");
  });

  it("honours an explicit tabIndex override", () => {
    render(
      <Button tabIndex={-1}>
        <Text>press</Text>
      </Button>,
    );
    expect(container.querySelector("[tabindex]")?.getAttribute("tabindex")).toBe("-1");
  });

  it("a disabled button is removed from the Tab order", () => {
    render(
      <Button disabled>
        <Text>press</Text>
      </Button>,
    );
    expect(container.querySelector("[tabindex]")).toBeNull();
  });

  it("a disabled button does not fire onClick", () => {
    const onClick = vi.fn();
    render(
      <Button disabled onClick={onClick}>
        <Text>press</Text>
      </Button>,
    );
    click("press");
    expect(onClick).not.toHaveBeenCalled();
  });

  // Activation comes from the runtime's synthesized click, not a key-down (double-fire guard).
  it("does not fire onClick on a raw Space/Enter key-down (avoids double activation)", () => {
    const onClick = vi.fn();
    render(
      <Button onClick={onClick}>
        <Text>press</Text>
      </Button>,
    );
    keydown("press", "space");
    keydown("press", "enter");
    expect(onClick).not.toHaveBeenCalled();
  });
});
