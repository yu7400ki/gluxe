// @vitest-environment jsdom
import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// The Tab trap itself lives in the runtime (core `scope_tab_target`, unit-tested
// in Rust); here we only check that FocusScope drives the focus lifecycle —
// trap, mount focus, restore — via the @gluxe/react/focus primitives.
const calls: string[] = [];
vi.mock("@gluxe/react/focus", () => ({
  getActiveElement: () => 7,
  focusElement: (id: number) => calls.push(`focusElement:${id}`),
  focusFirstElement: (id: number) => calls.push(`focusFirstElement:${id}`),
  pushTabScope: (id: number) => calls.push(`push:${id}`),
  popTabScope: (id: number) => calls.push(`pop:${id}`),
}));

import { FocusScope } from "./focus-scope";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let container: HTMLElement;
let root: Root;

beforeEach(() => {
  calls.length = 0;
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

// FocusScope reads `containerRef.current?.id`; the consumer attaches the ref, so
// here we just hand it a fixed instance.
function refTo(id: number): React.RefObject<{ id: number } | null> {
  return { current: { id } };
}

describe("FocusScope", () => {
  it("traps and focuses in on mount, releases and restores on unmount", () => {
    act(() =>
      root.render(
        <FocusScope containerRef={refTo(42) as never}>
          <span>body</span>
        </FocusScope>,
      ),
    );
    expect(calls).toEqual(["push:42", "focusFirstElement:42"]);

    calls.length = 0;
    act(() => root.unmount());
    expect(calls).toEqual(["pop:42", "focusElement:7"]);
  });

  it("does not trap when `trapped` is false (still focuses + restores)", () => {
    act(() =>
      root.render(
        <FocusScope containerRef={refTo(42) as never} trapped={false}>
          <span>body</span>
        </FocusScope>,
      ),
    );
    expect(calls).toEqual(["focusFirstElement:42"]);

    calls.length = 0;
    act(() => root.unmount());
    expect(calls).toEqual(["focusElement:7"]);
  });

  it("uses the mount/unmount overrides instead of the defaults", () => {
    const onMount = vi.fn();
    const onUnmount = vi.fn();
    act(() =>
      root.render(
        <FocusScope
          containerRef={refTo(42) as never}
          onMountAutoFocus={onMount}
          onUnmountAutoFocus={onUnmount}
        >
          <span>body</span>
        </FocusScope>,
      ),
    );
    // Still traps, but the override replaces focusFirstElement.
    expect(calls).toEqual(["push:42"]);
    expect(onMount).toHaveBeenCalledTimes(1);

    act(() => root.unmount());
    expect(onUnmount).toHaveBeenCalledWith(7);
  });
});
