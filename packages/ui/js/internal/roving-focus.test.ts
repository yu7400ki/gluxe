// Unit tests for the pure roving-focus navigation helpers.
import { describe, expect, it } from "vitest";

import { arrowDirection, nextEnabledIndex } from "./roving-focus";

describe("arrowDirection", () => {
  it("maps Home/End regardless of orientation", () => {
    expect(arrowDirection("home", "horizontal")).toBe("first");
    expect(arrowDirection("end", "vertical")).toBe("last");
    expect(arrowDirection("home", "both")).toBe("first");
  });

  it("horizontal responds to left/right only", () => {
    expect(arrowDirection("left", "horizontal")).toBe(-1);
    expect(arrowDirection("right", "horizontal")).toBe(1);
    expect(arrowDirection("up", "horizontal")).toBeNull();
    expect(arrowDirection("down", "horizontal")).toBeNull();
  });

  it("vertical responds to up/down only", () => {
    expect(arrowDirection("up", "vertical")).toBe(-1);
    expect(arrowDirection("down", "vertical")).toBe(1);
    expect(arrowDirection("left", "vertical")).toBeNull();
    expect(arrowDirection("right", "vertical")).toBeNull();
  });

  it("both responds to all four arrows", () => {
    expect(arrowDirection("left", "both")).toBe(-1);
    expect(arrowDirection("up", "both")).toBe(-1);
    expect(arrowDirection("right", "both")).toBe(1);
    expect(arrowDirection("down", "both")).toBe(1);
  });

  it("returns null for non-navigation keys", () => {
    expect(arrowDirection("enter", "both")).toBeNull();
    expect(arrowDirection("a", "horizontal")).toBeNull();
  });
});

describe("nextEnabledIndex", () => {
  const items = (flags: boolean[]) => flags.map((disabled) => ({ disabled }));

  it("steps forward to the next enabled item", () => {
    expect(nextEnabledIndex(items([false, false, false]), 0, 1, false)).toBe(1);
  });

  it("steps backward to the previous enabled item", () => {
    expect(nextEnabledIndex(items([false, false, false]), 2, -1, false)).toBe(1);
  });

  it("skips disabled items", () => {
    expect(nextEnabledIndex(items([false, true, false]), 0, 1, false)).toBe(2);
    expect(nextEnabledIndex(items([false, true, false]), 2, -1, false)).toBe(0);
  });

  it("returns null at the boundary when not looping", () => {
    expect(nextEnabledIndex(items([false, false]), 1, 1, false)).toBeNull();
    expect(nextEnabledIndex(items([false, false]), 0, -1, false)).toBeNull();
  });

  it("wraps around when looping", () => {
    expect(nextEnabledIndex(items([false, false]), 1, 1, true)).toBe(0);
    expect(nextEnabledIndex(items([false, false]), 0, -1, true)).toBe(1);
  });

  it("wraps past disabled items when looping", () => {
    expect(nextEnabledIndex(items([false, false, true]), 1, 1, true)).toBe(0);
  });

  it("finds the first enabled item from before the start", () => {
    expect(nextEnabledIndex(items([true, false, false]), -1, 1, false)).toBe(1);
  });

  it("finds the last enabled item from past the end", () => {
    expect(nextEnabledIndex(items([false, false, true]), 3, -1, false)).toBe(1);
  });

  it("returns null when every item is disabled", () => {
    expect(nextEnabledIndex(items([true, true]), 0, 1, true)).toBeNull();
  });

  it("returns null for an empty list", () => {
    expect(nextEnabledIndex([], 0, 1, true)).toBeNull();
  });
});
