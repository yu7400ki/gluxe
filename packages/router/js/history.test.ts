import { describe, expect, it, vi } from "vitest";

import { createMemoryHistory } from "./history";

describe("createMemoryHistory", () => {
  it("starts at '/' by default", () => {
    const history = createMemoryHistory();
    expect(history.location.pathname).toBe("/");
    expect(history.index).toBe(0);
    expect(history.length).toBe(1);
  });

  it("uses the last initial entry as the current location", () => {
    const history = createMemoryHistory(["/", "/about"]);
    expect(history.location.pathname).toBe("/about");
    expect(history.index).toBe(1);
    expect(history.length).toBe(2);
  });

  it("falls back to '/' for an empty initialEntries array", () => {
    const history = createMemoryHistory([]);
    expect(history.location.pathname).toBe("/");
  });

  it("normalizes pathnames (leading slash added, trailing slash stripped)", () => {
    const history = createMemoryHistory();
    history.push("about");
    expect(history.location.pathname).toBe("/about");
    history.push("/users/42/");
    expect(history.location.pathname).toBe("/users/42");
    history.push("/");
    expect(history.location.pathname).toBe("/");
  });

  it("push appends an entry and moves the index", () => {
    const history = createMemoryHistory();
    history.push("/a");
    history.push("/b");
    expect(history.location.pathname).toBe("/b");
    expect(history.index).toBe(2);
    expect(history.length).toBe(3);
  });

  it("push discards forward entries", () => {
    const history = createMemoryHistory();
    history.push("/a");
    history.push("/b");
    history.back();
    history.back();
    expect(history.location.pathname).toBe("/");
    history.push("/c");
    expect(history.location.pathname).toBe("/c");
    expect(history.length).toBe(2);
    history.forward(); // no forward entries remain
    expect(history.location.pathname).toBe("/c");
  });

  it("replace swaps the current entry without growing the stack", () => {
    const history = createMemoryHistory();
    history.push("/a");
    history.replace("/b");
    expect(history.location.pathname).toBe("/b");
    expect(history.index).toBe(1);
    expect(history.length).toBe(2);
    history.back();
    expect(history.location.pathname).toBe("/");
  });

  it("go clamps to the stack bounds", () => {
    const history = createMemoryHistory(["/", "/a", "/b"]);
    history.go(-99);
    expect(history.index).toBe(0);
    history.go(99);
    expect(history.index).toBe(2);
  });

  it("back and forward move through the stack", () => {
    const history = createMemoryHistory(["/", "/a", "/b"]);
    history.back();
    expect(history.location.pathname).toBe("/a");
    history.forward();
    expect(history.location.pathname).toBe("/b");
  });

  it("notifies listeners on navigation, but not on clamped no-op moves", () => {
    const history = createMemoryHistory();
    const listener = vi.fn();
    history.listen(listener);
    history.push("/a");
    expect(listener).toHaveBeenCalledTimes(1);
    history.replace("/b");
    expect(listener).toHaveBeenCalledTimes(2);
    history.back();
    expect(listener).toHaveBeenCalledTimes(3);
    history.back(); // already at index 0
    expect(listener).toHaveBeenCalledTimes(3);
    history.go(0);
    expect(listener).toHaveBeenCalledTimes(3);
  });

  it("listen returns an unsubscribe function", () => {
    const history = createMemoryHistory();
    const listener = vi.fn();
    const unsubscribe = history.listen(listener);
    unsubscribe();
    history.push("/a");
    expect(listener).not.toHaveBeenCalled();
  });

  it("gives every entry a fresh key and a fresh location identity", () => {
    const history = createMemoryHistory();
    const first = history.location;
    history.push("/a");
    history.replace("/a");
    expect(history.location).not.toBe(first);
    expect(history.location.key).not.toBe(first.key);
  });

  it("supports destructured methods (no this dependency)", () => {
    const history = createMemoryHistory(["/", "/a", "/b"]);
    const { back, forward, go, push } = history;
    back();
    expect(history.location.pathname).toBe("/a");
    forward();
    expect(history.location.pathname).toBe("/b");
    go(-2);
    expect(history.location.pathname).toBe("/");
    push("/c");
    expect(history.location.pathname).toBe("/c");
  });

  it("stores navigation state on the location", () => {
    const history = createMemoryHistory();
    history.push("/a", { from: "test" });
    expect(history.location.state).toEqual({ from: "test" });
    history.back();
    expect(history.location.state).toBeUndefined();
  });
});
