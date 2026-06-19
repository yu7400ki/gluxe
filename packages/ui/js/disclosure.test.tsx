// @vitest-environment jsdom
//
// Tests for the Disclosure headless component.
import { Text } from "@gluxe/react";
import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Disclosure } from "./disclosure";

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

describe("Disclosure", () => {
  it("content is unmounted when closed by default", () => {
    render(
      <Disclosure defaultOpen={false}>
        <Disclosure.Trigger>
          <Text>toggle</Text>
        </Disclosure.Trigger>
        <Disclosure.Content>
          <Text>body</Text>
        </Disclosure.Content>
      </Disclosure>,
    );
    expect(container.textContent).not.toContain("body");
  });

  it("content mounts when open", () => {
    render(
      <Disclosure defaultOpen={true}>
        <Disclosure.Trigger>
          <Text>toggle</Text>
        </Disclosure.Trigger>
        <Disclosure.Content>
          <Text>body</Text>
        </Disclosure.Content>
      </Disclosure>,
    );
    expect(container.textContent).toContain("body");
  });

  it("trigger toggles content open", () => {
    render(
      <Disclosure defaultOpen={false}>
        <Disclosure.Trigger>
          <Text>toggle</Text>
        </Disclosure.Trigger>
        <Disclosure.Content>
          <Text>body</Text>
        </Disclosure.Content>
      </Disclosure>,
    );
    expect(container.textContent).not.toContain("body");
    click("toggle");
    expect(container.textContent).toContain("body");
  });

  it("trigger toggles content closed again", () => {
    render(
      <Disclosure defaultOpen={true}>
        <Disclosure.Trigger>
          <Text>toggle</Text>
        </Disclosure.Trigger>
        <Disclosure.Content>
          <Text>body</Text>
        </Disclosure.Content>
      </Disclosure>,
    );
    expect(container.textContent).toContain("body");
    click("toggle");
    expect(container.textContent).not.toContain("body");
  });

  it("onOpenChange fires with the next open state", () => {
    const onChange = vi.fn();
    render(
      <Disclosure defaultOpen={false} onOpenChange={onChange}>
        <Disclosure.Trigger>
          <Text>toggle</Text>
        </Disclosure.Trigger>
      </Disclosure>,
    );
    click("toggle");
    expect(onChange).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledWith(true);

    click("toggle");
    expect(onChange).toHaveBeenCalledTimes(2);
    expect(onChange).toHaveBeenLastCalledWith(false);
  });

  it("trigger render-prop exposes open state", () => {
    render(
      <Disclosure defaultOpen={false}>
        <Disclosure.Trigger>
          {({ open }) => <Text>{open ? "close" : "open"}</Text>}
        </Disclosure.Trigger>
        <Disclosure.Content>
          <Text>body</Text>
        </Disclosure.Content>
      </Disclosure>,
    );
    expect(container.textContent).toContain("open");
    click("open");
    expect(container.textContent).toContain("close");
  });

  it("disabled blocks the trigger from toggling", () => {
    render(
      <Disclosure defaultOpen={false} disabled>
        <Disclosure.Trigger>
          <Text>toggle</Text>
        </Disclosure.Trigger>
        <Disclosure.Content>
          <Text>body</Text>
        </Disclosure.Content>
      </Disclosure>,
    );
    click("toggle");
    expect(container.textContent).not.toContain("body");
  });

  it("disabled prevents onOpenChange from firing", () => {
    const onChange = vi.fn();
    render(
      <Disclosure defaultOpen={false} disabled onOpenChange={onChange}>
        <Disclosure.Trigger>
          <Text>toggle</Text>
        </Disclosure.Trigger>
      </Disclosure>,
    );
    click("toggle");
    expect(onChange).not.toHaveBeenCalled();
  });
});
