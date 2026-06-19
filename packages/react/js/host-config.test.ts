// Characterization tests for host-config.ts pure helpers.
//
// We do NOT mount any React tree here — jsdom + react-reconciler crashes
// (known gotcha). Instead we test the pure extraction logic and the
// __dispatchEvent routing entirely in isolation by mocking globalThis.__bridge.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Install a mock __bridge BEFORE importing host-config (which reads
// `hostGlobal.__bridge` at module-eval time to build `bridge`).
// Because vitest module isolation runs per-file, this assignment lands
// before the module is first evaluated.
const mockBridge = {
  createInstance: vi.fn(() => 42),
  createText: vi.fn(() => 99),
  appendChild: vi.fn(),
  appendToContainer: vi.fn(),
  insertBefore: vi.fn(),
  insertInContainer: vi.fn(),
  removeChild: vi.fn(),
  removeFromContainer: vi.fn(),
  updateProps: vi.fn(),
  updateText: vi.fn(),
  clearContainer: vi.fn(),
  detachDeleted: vi.fn(),
};
(globalThis as Record<string, unknown>).__bridge = mockBridge;

// Also stub __invoke so importing invoke.ts (pulled in by host-config.ts) does
// not throw at call time.
(globalThis as Record<string, unknown>).__invoke = vi.fn();

// Now import the module under test (after the globals are in place).
import { EVENT_PROP_TO_TYPE, extractHandlers, handlers } from "./host-config";

// ─── EVENT_PROP_TO_TYPE ─────────────────────────────────────────────────────

describe("EVENT_PROP_TO_TYPE mapping", () => {
  it("maps onClick → click", () => {
    expect(EVENT_PROP_TO_TYPE["onClick"]).toBe("click");
  });

  it("maps onMouseDown → mousedown", () => {
    expect(EVENT_PROP_TO_TYPE["onMouseDown"]).toBe("mousedown");
  });

  it("maps onMouseUp → mouseup", () => {
    expect(EVENT_PROP_TO_TYPE["onMouseUp"]).toBe("mouseup");
  });

  it("maps onMouseMove → mousemove", () => {
    expect(EVENT_PROP_TO_TYPE["onMouseMove"]).toBe("mousemove");
  });

  it("maps onMouseEnter → mouseenter", () => {
    expect(EVENT_PROP_TO_TYPE["onMouseEnter"]).toBe("mouseenter");
  });

  it("maps onMouseLeave → mouseleave", () => {
    expect(EVENT_PROP_TO_TYPE["onMouseLeave"]).toBe("mouseleave");
  });

  it("maps onKeyDown → keydown", () => {
    expect(EVENT_PROP_TO_TYPE["onKeyDown"]).toBe("keydown");
  });

  it("maps onChangeText → change", () => {
    expect(EVENT_PROP_TO_TYPE["onChangeText"]).toBe("change");
  });

  it("maps onSubmit → submit", () => {
    expect(EVENT_PROP_TO_TYPE["onSubmit"]).toBe("submit");
  });

  it("maps onFocus → focus", () => {
    expect(EVENT_PROP_TO_TYPE["onFocus"]).toBe("focus");
  });

  it("maps onBlur → blur", () => {
    expect(EVENT_PROP_TO_TYPE["onBlur"]).toBe("blur");
  });

  it("has exactly 11 entries (no surprise extras)", () => {
    expect(Object.keys(EVENT_PROP_TO_TYPE)).toHaveLength(11);
  });
});

// ─── extractHandlers ─────────────────────────────────────────────────────────

describe("extractHandlers", () => {
  it("separates event handler props from plain props", () => {
    const clickFn = vi.fn();
    const result = extractHandlers({
      style: { backgroundColor: "red" },
      onClick: clickFn,
      children: null,
    });

    expect(result.plain).toEqual({ style: { backgroundColor: "red" }, children: null });
    expect(result.events).toEqual(["click"]);
    expect(result.map).toEqual({ click: clickFn });
  });

  it("puts non-function values for event-prop-named keys into plain (not treated as handlers)", () => {
    // onClick that is a string, not a function — must go to plain
    const result = extractHandlers({ onClick: "not-a-function" });

    expect(result.plain).toEqual({ onClick: "not-a-function" });
    expect(result.events).toHaveLength(0);
    expect(result.map).toEqual({});
  });

  it("handles multiple event props at once", () => {
    const clickFn = vi.fn();
    const keyFn = vi.fn();
    const result = extractHandlers({ onClick: clickFn, onKeyDown: keyFn, tabIndex: 0 });

    expect(result.events).toContain("click");
    expect(result.events).toContain("keydown");
    expect(result.events).toHaveLength(2);
    expect(result.map["click"]).toBe(clickFn);
    expect(result.map["keydown"]).toBe(keyFn);
    expect(result.plain).toEqual({ tabIndex: 0 });
  });

  it("returns empty events/map for props with no handlers", () => {
    const result = extractHandlers({ style: {}, tabIndex: 0 });

    expect(result.events).toHaveLength(0);
    expect(result.map).toEqual({});
    expect(result.plain).toEqual({ style: {}, tabIndex: 0 });
  });

  it("maps onChangeText to event type 'change'", () => {
    const fn = vi.fn();
    const result = extractHandlers({ onChangeText: fn });
    expect(result.events).toEqual(["change"]);
    expect(result.map["change"]).toBe(fn);
  });

  it("maps onSubmit to event type 'submit'", () => {
    const fn = vi.fn();
    const result = extractHandlers({ onSubmit: fn });
    expect(result.events).toEqual(["submit"]);
    expect(result.map["submit"]).toBe(fn);
  });

  it("maps onFocus to 'focus' and onBlur to 'blur'", () => {
    const focusFn = vi.fn();
    const blurFn = vi.fn();
    const result = extractHandlers({ onFocus: focusFn, onBlur: blurFn });
    expect(result.events).toContain("focus");
    expect(result.events).toContain("blur");
    expect(result.map["focus"]).toBe(focusFn);
    expect(result.map["blur"]).toBe(blurFn);
  });

  it("returns empty objects for empty props", () => {
    const result = extractHandlers({});
    expect(result.plain).toEqual({});
    expect(result.events).toHaveLength(0);
    expect(result.map).toEqual({});
  });
});

// ─── __dispatchEvent routing ─────────────────────────────────────────────────

interface HostGlobalWithDispatch {
  __dispatchEvent?: (id: number, type: string, payload: Record<string, unknown>) => void;
}

const hostGlobal = globalThis as unknown as HostGlobalWithDispatch;

describe("__dispatchEvent routing", () => {
  const TEST_ID = 7;

  beforeEach(() => {
    handlers.clear();
  });

  afterEach(() => {
    handlers.clear();
  });

  it("routes a click event to the registered handler with spread payload", () => {
    const clickFn = vi.fn();
    handlers.set(TEST_ID, { click: clickFn });

    hostGlobal.__dispatchEvent!(TEST_ID, "click", { x: 10, y: 20 });

    expect(clickFn).toHaveBeenCalledOnce();
    expect(clickFn).toHaveBeenCalledWith({ type: "click", target: TEST_ID, x: 10, y: 20 });
  });

  it("routes a keydown event with all modifier fields spread in", () => {
    const keyFn = vi.fn();
    handlers.set(TEST_ID, { keydown: keyFn });

    hostGlobal.__dispatchEvent!(TEST_ID, "keydown", {
      key: "enter",
      shift: false,
      ctrl: false,
      alt: false,
      meta: false,
    });

    expect(keyFn).toHaveBeenCalledWith({
      type: "keydown",
      target: TEST_ID,
      key: "enter",
      shift: false,
      ctrl: false,
      alt: false,
      meta: false,
    });
  });

  it("delivers 'change' as raw string (RN convention), not an event object", () => {
    const changeFn = vi.fn();
    handlers.set(TEST_ID, { change: changeFn });

    hostGlobal.__dispatchEvent!(TEST_ID, "change", { value: "hello" });

    expect(changeFn).toHaveBeenCalledOnce();
    expect(changeFn).toHaveBeenCalledWith("hello");
  });

  it("delivers 'submit' as raw string (RN convention)", () => {
    const submitFn = vi.fn();
    handlers.set(TEST_ID, { submit: submitFn });

    hostGlobal.__dispatchEvent!(TEST_ID, "submit", { value: "sent text" });

    expect(submitFn).toHaveBeenCalledWith("sent text");
  });

  it("delivers empty string when change/submit payload.value is not a string", () => {
    const changeFn = vi.fn();
    handlers.set(TEST_ID, { change: changeFn });

    hostGlobal.__dispatchEvent!(TEST_ID, "change", { value: 42 });

    expect(changeFn).toHaveBeenCalledWith("");
  });

  it("routes focus event as a normal event object (not raw-string path)", () => {
    const focusFn = vi.fn();
    handlers.set(TEST_ID, { focus: focusFn });

    hostGlobal.__dispatchEvent!(TEST_ID, "focus", { value: "some text" });

    expect(focusFn).toHaveBeenCalledWith({ type: "focus", target: TEST_ID, value: "some text" });
  });

  it("routes blur event as a normal event object", () => {
    const blurFn = vi.fn();
    handlers.set(TEST_ID, { blur: blurFn });

    hostGlobal.__dispatchEvent!(TEST_ID, "blur", { value: "" });

    expect(blurFn).toHaveBeenCalledWith({ type: "blur", target: TEST_ID, value: "" });
  });

  it("is a safe no-op for an unknown element id (no throw)", () => {
    expect(() => {
      hostGlobal.__dispatchEvent!(9999, "click", { x: 0, y: 0 });
    }).not.toThrow();
  });

  it("is a safe no-op for an unknown event type on a registered element (no throw)", () => {
    handlers.set(TEST_ID, { click: vi.fn() });

    expect(() => {
      hostGlobal.__dispatchEvent!(TEST_ID, "unknown-event", {});
    }).not.toThrow();
  });

  it("does not call a handler for a different element id", () => {
    const fn = vi.fn();
    handlers.set(TEST_ID, { click: fn });

    hostGlobal.__dispatchEvent!(TEST_ID + 1, "click", { x: 0, y: 0 });

    expect(fn).not.toHaveBeenCalled();
  });
});
