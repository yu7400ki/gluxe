// Characterization tests for stream.ts — GluxeStream and invokeStream().
//
// We drive __streamPush (installed by the module) to push envelopes,
// and mock __invokeStream / __streamCancel so no Rust bridge is needed.

import { beforeEach, describe, expect, it, vi } from "vitest";

// Install mock globals BEFORE importing stream.ts (lazy reads pick them up).
const mockInvokeStream = vi.fn();
const mockStreamCancel = vi.fn();
(globalThis as Record<string, unknown>).__invokeStream = mockInvokeStream;
(globalThis as Record<string, unknown>).__streamCancel = mockStreamCancel;

// Import after globals are set. This also installs __streamPush on globalThis.
import { GluxeStream, invokeStream } from "./stream";

interface StreamGlobal {
  __streamPush?: (streamId: number, json: string) => void;
}
const streamGlobal = globalThis as unknown as StreamGlobal;

// Helper: encode an envelope as the pump loop would.
function pushChunk(streamId: number, value: unknown) {
  streamGlobal.__streamPush!(streamId, JSON.stringify({ t: "chunk", value }));
}
function pushEnd(streamId: number) {
  streamGlobal.__streamPush!(streamId, JSON.stringify({ t: "end" }));
}
function pushError(streamId: number, error?: string) {
  streamGlobal.__streamPush!(streamId, JSON.stringify({ t: "error", error }));
}

// ─── GluxeStream unit tests (constructed directly, bypassing invokeStream) ───

describe("GluxeStream", () => {
  describe("queue: chunk arrives BEFORE reader awaits", () => {
    it("delivers buffered chunk immediately on read()", async () => {
      const stream = new GluxeStream<string>(0);
      stream._enqueue("hello");

      const reader = stream.getReader();
      const result = await reader.read();
      expect(result).toEqual({ value: "hello", done: false });
    });

    it("delivers multiple buffered chunks in order", async () => {
      const stream = new GluxeStream<number>(0);
      stream._enqueue(1);
      stream._enqueue(2);
      stream._enqueue(3);

      const reader = stream.getReader();
      expect(await reader.read()).toEqual({ value: 1, done: false });
      expect(await reader.read()).toEqual({ value: 2, done: false });
      expect(await reader.read()).toEqual({ value: 3, done: false });
    });

    it("buffers chunks if no waiter, then delivers on subsequent reads", async () => {
      const stream = new GluxeStream<string>(0);
      stream._enqueue("a");
      stream._enqueue("b");

      const reader = stream.getReader();
      const r1 = await reader.read();
      const r2 = await reader.read();
      expect(r1.value).toBe("a");
      expect(r2.value).toBe("b");
    });
  });

  describe("waiter: reader awaits BEFORE chunk arrives", () => {
    it("resolves the pending read() when a chunk is enqueued", async () => {
      const stream = new GluxeStream<string>(0);
      const reader = stream.getReader();

      const readPromise = reader.read();
      // Enqueue after the promise is parked
      stream._enqueue("world");

      const result = await readPromise;
      expect(result).toEqual({ value: "world", done: false });
    });

    it("resolves multiple parked waiters in order", async () => {
      const stream = new GluxeStream<number>(0);
      const reader = stream.getReader();

      const p1 = reader.read();
      const p2 = reader.read();

      stream._enqueue(10);
      stream._enqueue(20);

      expect(await p1).toEqual({ value: 10, done: false });
      expect(await p2).toEqual({ value: 20, done: false });
    });
  });

  describe("_close()", () => {
    it("resolves a pending read() with done:true", async () => {
      const stream = new GluxeStream<string>(0);
      const reader = stream.getReader();

      const readPromise = reader.read();
      stream._close();

      const result = await readPromise;
      expect(result.done).toBe(true);
    });

    it("immediately returns done:true for reads after close", async () => {
      const stream = new GluxeStream<string>(0);
      stream._close();

      const reader = stream.getReader();
      const result = await reader.read();
      expect(result.done).toBe(true);
    });

    it("calling _close() a second time is a no-op (idempotent)", async () => {
      const stream = new GluxeStream<string>(0);
      stream._close();
      // Should not throw
      expect(() => stream._close()).not.toThrow();
    });

    it("delivers queued chunks before done after close (via sequential reads)", async () => {
      const stream = new GluxeStream<number>(0);
      stream._enqueue(42);
      stream._close();

      const reader = stream.getReader();
      const r1 = await reader.read();
      const r2 = await reader.read();
      expect(r1).toEqual({ value: 42, done: false });
      expect(r2.done).toBe(true);
    });
  });

  describe("_error()", () => {
    it("rejects a pending read() with the error", async () => {
      const stream = new GluxeStream<string>(0);
      const reader = stream.getReader();

      const readPromise = reader.read();
      stream._error(new Error("boom"));

      await expect(readPromise).rejects.toThrow("boom");
    });

    it("calling _error() after _close() is a no-op (second terminal ignored)", () => {
      const stream = new GluxeStream<string>(0);
      stream._close();
      // Must not throw
      expect(() => stream._error(new Error("late"))).not.toThrow();
    });

    it("calling _close() after _error() is a no-op", () => {
      const stream = new GluxeStream<string>(0);
      stream._error(new Error("first"));
      expect(() => stream._close()).not.toThrow();
    });

    it("calling _error() a second time is a no-op", () => {
      const stream = new GluxeStream<string>(0);
      stream._error(new Error("first"));
      expect(() => stream._error(new Error("second"))).not.toThrow();
    });

    it("error is surfaced once via the pull() err field, then cleared", async () => {
      const stream = new GluxeStream<string>(0);
      stream._error(new Error("once"));

      const reader = stream.getReader();
      // First read should reject.
      await expect(reader.read()).rejects.toThrow("once");
      // Second read: done is true, so it resolves as done (not rejects again).
      const r2 = await reader.read();
      expect(r2.done).toBe(true);
    });
  });

  describe("cancel()", () => {
    it("is idempotent — calling cancel twice does not throw", () => {
      const stream = new GluxeStream<string>(100);
      stream.cancel();
      expect(() => stream.cancel()).not.toThrow();
    });

    it("after cancel, _enqueue is a no-op (chunks are dropped)", () => {
      const stream = new GluxeStream<string>(0);
      stream.cancel();
      // Should not throw, and no waiter to resolve
      expect(() => stream._enqueue("dropped")).not.toThrow();
    });

    it("after cancel, done:true is returned for subsequent reads", async () => {
      const stream = new GluxeStream<string>(0);
      stream.cancel();
      const reader = stream.getReader();
      const result = await reader.read();
      expect(result.done).toBe(true);
    });

    it("reader.cancel() is also idempotent", async () => {
      const stream = new GluxeStream<string>(0);
      const reader = stream.getReader();
      await reader.cancel();
      await expect(reader.cancel()).resolves.toBeUndefined();
    });
  });

  describe("async iterator", () => {
    it("iterates all enqueued chunks then terminates on _close()", async () => {
      const stream = new GluxeStream<number>(0);
      const collected: number[] = [];

      stream._enqueue(1);
      stream._enqueue(2);
      stream._close();

      for await (const chunk of stream) {
        collected.push(chunk);
      }

      expect(collected).toEqual([1, 2]);
    });

    it("break causes cancel via iterator return()", async () => {
      const stream = new GluxeStream<number>(0);
      // Provide a chunk so the loop can enter, then break.
      const iterPromise = (async () => {
        stream._enqueue(1);
        for await (const _chunk of stream) {
          break;
        }
      })();
      await iterPromise;
      // After break, stream should be cancelled (done=true)
      const result = await stream.getReader().read();
      expect(result.done).toBe(true);
    });
  });
});

// ─── invokeStream() + __streamPush integration ───────────────────────────────

describe("invokeStream() via __streamPush", () => {
  beforeEach(() => {
    mockInvokeStream.mockClear();
    mockStreamCancel.mockClear();
  });

  it("calls __invokeStream with a monotonic stream id, cmd, and JSON args", () => {
    invokeStream("fs|readFileStream", { path: "/tmp/data" });

    expect(mockInvokeStream).toHaveBeenCalledOnce();
    const [id, cmd, argsJson] = mockInvokeStream.mock.calls[0] as [number, string, string];
    expect(typeof id).toBe("number");
    expect(id).toBeGreaterThan(0);
    expect(cmd).toBe("fs|readFileStream");
    expect(JSON.parse(argsJson)).toEqual({ path: "/tmp/data" });
  });

  it("delivers a chunk pushed via __streamPush (chunk envelope)", async () => {
    const stream = invokeStream<string>("test|cmd");
    const [streamId] = mockInvokeStream.mock.calls[0] as [number];

    const readPromise = stream.getReader().read();
    pushChunk(streamId, "pushed value");

    const result = await readPromise;
    expect(result).toEqual({ value: "pushed value", done: false });
  });

  it("closes the stream on an 'end' envelope", async () => {
    const stream = invokeStream<string>("test|cmd");
    const [streamId] = mockInvokeStream.mock.calls[0] as [number];

    const readPromise = stream.getReader().read();
    pushEnd(streamId);

    const result = await readPromise;
    expect(result.done).toBe(true);
  });

  it("errors the stream on an 'error' envelope with a message", async () => {
    const stream = invokeStream<string>("test|cmd");
    const [streamId] = mockInvokeStream.mock.calls[0] as [number];

    const readPromise = stream.getReader().read();
    pushError(streamId, "native error");

    await expect(readPromise).rejects.toThrow("native error");
  });

  it("uses default error message when error envelope has no error field", async () => {
    const stream = invokeStream<string>("test|cmd");
    const [streamId] = mockInvokeStream.mock.calls[0] as [number];

    const readPromise = stream.getReader().read();
    pushError(streamId);

    await expect(readPromise).rejects.toThrow("stream failed");
  });

  it("a late __streamPush to a terminated stream id is a safe no-op", () => {
    invokeStream<string>("test|cmd");
    const [streamId] = mockInvokeStream.mock.calls[0] as [number];

    pushEnd(streamId);

    // After end, the controller is removed; another push should not throw.
    expect(() => pushChunk(streamId, "late")).not.toThrow();
  });

  it("cancelling calls __streamCancel with the correct stream id", () => {
    const stream = invokeStream<string>("test|cmd");
    const [streamId] = mockInvokeStream.mock.calls[0] as [number];

    stream.cancel();

    expect(mockStreamCancel).toHaveBeenCalledWith(streamId);
  });

  it("multiple concurrent streams receive independent chunks", async () => {
    const s1 = invokeStream<string>("a|cmd");
    const s2 = invokeStream<string>("b|cmd");
    const [[id1], [id2]] = mockInvokeStream.mock.calls as [number, string, string][];

    const r1 = s1.getReader().read();
    const r2 = s2.getReader().read();

    pushChunk(id2!, "for-s2");
    pushChunk(id1!, "for-s1");

    expect(await r1).toEqual({ value: "for-s1", done: false });
    expect(await r2).toEqual({ value: "for-s2", done: false });
  });
});
