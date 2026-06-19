// @vitest-environment jsdom
//
// Tests for the Portal headless component.
import { Text } from "@gluxe/react";
import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Portal } from "./portal";

// `createPortal` routes through the gluxe reconciler (needs the native bridge),
// which cannot commit under react-dom/jsdom. Stub it to render children inline so
// we can verify Portal forwards its children through it.
const createPortal = vi.fn((children: React.ReactNode) => children);
vi.mock("@gluxe/react", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@gluxe/react")>();
  return { ...actual, createPortal: (children: React.ReactNode) => createPortal(children) };
});

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let container: HTMLElement;
let root: Root;

beforeEach(() => {
  createPortal.mockClear();
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

describe("Portal", () => {
  it("renders its children (delegating to createPortal)", () => {
    render(
      <Portal>
        <Text>portaled</Text>
      </Portal>,
    );
    expect(container.textContent).toContain("portaled");
    expect(createPortal).toHaveBeenCalledOnce();
  });
});
