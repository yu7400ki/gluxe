// Characterization tests for invoke.ts.
//
// We mock globalThis.__invoke and drive globalThis.__resolveInvoke
// (which is installed by the module itself) to settle Promises.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Install mock __invoke BEFORE importing invoke.ts so the lazy read picks it up.
const mockNativeInvoke = vi.fn();
(globalThis as Record<string, unknown>).__invoke = mockNativeInvoke;

// Import after setting up the global. This also installs __resolveInvoke on globalThis.
import { invoke, pending } from "./invoke";

interface InvokeGlobal {
  __resolveInvoke?: (id: number, json: string) => void;
}
const invokeGlobal = globalThis as unknown as InvokeGlobal;

describe("invoke()", () => {
  beforeEach(() => {
    mockNativeInvoke.mockClear();
    pending.clear();
  });

  afterEach(() => {
    pending.clear();
  });

  it("calls __invoke with a monotonic id, the command key, and JSON-encoded args", () => {
    void invoke("fs|readTextFile", { path: "/tmp/x.txt" });

    expect(mockNativeInvoke).toHaveBeenCalledOnce();
    const [id, cmd, argsJson] = mockNativeInvoke.mock.calls[0] as [number, string, string];
    expect(typeof id).toBe("number");
    expect(id).toBeGreaterThan(0);
    expect(cmd).toBe("fs|readTextFile");
    expect(JSON.parse(argsJson)).toEqual({ path: "/tmp/x.txt" });
  });

  it("resolves with the value when __resolveInvoke is called with ok:true", async () => {
    const promise = invoke<string>("fs|readTextFile", { path: "/tmp/x.txt" });
    const [id] = mockNativeInvoke.mock.calls[0] as [number, string, string];

    invokeGlobal.__resolveInvoke!(id, JSON.stringify({ ok: true, value: "file contents" }));

    await expect(promise).resolves.toBe("file contents");
  });

  it("rejects with an Error when __resolveInvoke is called with ok:false", async () => {
    const promise = invoke("fs|readTextFile", { path: "/nonexistent" });
    const [id] = mockNativeInvoke.mock.calls[0] as [number, string, string];

    invokeGlobal.__resolveInvoke!(id, JSON.stringify({ ok: false, error: "file not found" }));

    await expect(promise).rejects.toThrow("file not found");
  });

  it("rejects with a default message when ok:false has no error field", async () => {
    const promise = invoke("some|cmd");
    const [id] = mockNativeInvoke.mock.calls[0] as [number, string, string];

    invokeGlobal.__resolveInvoke!(id, JSON.stringify({ ok: false }));

    await expect(promise).rejects.toThrow("command failed");
  });

  it("uses default empty args when args parameter is omitted", () => {
    void invoke("some|cmd");
    const [, , argsJson] = mockNativeInvoke.mock.calls[0] as [number, string, string];
    expect(JSON.parse(argsJson)).toEqual({});
  });

  it("resolving an unknown id is a safe no-op (does not throw)", () => {
    expect(() => {
      invokeGlobal.__resolveInvoke!(99999, JSON.stringify({ ok: true, value: null }));
    }).not.toThrow();
  });

  it("two concurrent invokes settle independently", async () => {
    const p1 = invoke<number>("math|add", { a: 1, b: 2 });
    const p2 = invoke<number>("math|mul", { a: 3, b: 4 });

    const [[id1], [id2]] = mockNativeInvoke.mock.calls as [number, string, string][];

    // Settle them in reverse order.
    invokeGlobal.__resolveInvoke!(id2, JSON.stringify({ ok: true, value: 12 }));
    invokeGlobal.__resolveInvoke!(id1, JSON.stringify({ ok: true, value: 3 }));

    await expect(p1).resolves.toBe(3);
    await expect(p2).resolves.toBe(12);
  });

  it("ids are monotonically increasing across calls", () => {
    void invoke("a|b");
    void invoke("c|d");
    void invoke("e|f");

    const ids = (mockNativeInvoke.mock.calls as [number, string, string][]).map(([id]) => id);
    for (let i = 1; i < ids.length; i++) {
      expect(ids[i]).toBeGreaterThan(ids[i - 1]!);
    }
  });

  it("removes the pending entry after resolution (no memory leak)", async () => {
    const promise = invoke<null>("noop|cmd");
    const [id] = mockNativeInvoke.mock.calls[0] as [number, string, string];

    expect(pending.has(id)).toBe(true);
    invokeGlobal.__resolveInvoke!(id, JSON.stringify({ ok: true, value: null }));
    await promise;
    expect(pending.has(id)).toBe(false);
  });
});
