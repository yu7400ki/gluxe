// Unit tests for the pure roving-focus navigation helpers.
import { describe, expect, it } from "vitest";

import { arrowDirection, nextEnabledIndex, type RovingItem, typeaheadMatch } from "./roving-focus";

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

describe("typeaheadMatch", () => {
  const item = (
    value: string,
    opts: { disabled?: boolean; textValue?: string } = {},
  ): RovingItem => ({
    value,
    disabled: opts.disabled ?? false,
    textValue: opts.textValue,
    focus: () => {},
  });

  const fruits = [item("apple"), item("apricot"), item("banana"), item("cherry")];

  it("matches the first option with the typed prefix", () => {
    expect(typeaheadMatch(fruits, undefined, "b")).toBe("banana");
  });

  it("is case-insensitive", () => {
    expect(typeaheadMatch(fruits, undefined, "B")).toBe("banana");
    expect(typeaheadMatch([item("Banana")], undefined, "ba")).toBe("Banana");
  });

  it("skips disabled options", () => {
    const list = [item("apple"), item("banana", { disabled: true }), item("blueberry")];
    expect(typeaheadMatch(list, undefined, "b")).toBe("blueberry");
  });

  it("wraps from the current option back to the start", () => {
    // Current = cherry (last); typing "a" wraps round to apple.
    expect(typeaheadMatch(fruits, "cherry", "a")).toBe("apple");
  });

  it("cycles same-prefix options on a single / repeated char (advances past current)", () => {
    expect(typeaheadMatch(fruits, "apple", "a")).toBe("apricot");
    // A repeated char ("aa") matches just "a" and keeps advancing → wraps to apple.
    expect(typeaheadMatch(fruits, "apricot", "aa")).toBe("apple");
  });

  it("refines in place for a distinct multi-char prefix (does not advance)", () => {
    // apricot still matches "ap" → stays put rather than jumping to apple.
    expect(typeaheadMatch(fruits, "apricot", "ap")).toBe("apricot");
    // From a non-matching current, a multi-char prefix searches forward.
    expect(typeaheadMatch(fruits, "banana", "ap")).toBe("apple");
  });

  it("returns null when nothing matches", () => {
    expect(typeaheadMatch(fruits, undefined, "z")).toBeNull();
  });

  it("returns null for an empty buffer", () => {
    expect(typeaheadMatch(fruits, "apple", "")).toBeNull();
  });

  it("matches textValue when it differs from value", () => {
    const list = [item("us", { textValue: "United States" }), item("ca", { textValue: "Canada" })];
    expect(typeaheadMatch(list, undefined, "ca")).toBe("ca");
    expect(typeaheadMatch(list, undefined, "u")).toBe("us");
  });

  it("falls back to index 0 when the current value is unknown", () => {
    expect(typeaheadMatch(fruits, "nonexistent", "a")).toBe("apple");
  });
});
