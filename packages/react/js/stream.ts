// Streaming native command bridge. The multi-chunk analogue of `invoke`:
// `__invokeStream` is fire-and-forget; Rust pushes chunks over time via
// `__streamPush(streamId, json)` from the pump loop, terminating with an
// `end` or `error` envelope. JS cancellation flows back through
// `__streamCancel(streamId)` (cooperative — the Rust handler polls it).
//
// Boa has no WHATWG `ReadableStream`, so `GluxeStream` is a minimal,
// self-contained implementation covering the surface we need: `getReader()`,
// async iteration (`for await`), and `cancel()`. Chunks carry arbitrary JSON
// values (binary data should be base64-encoded by the plugin and decoded here).

type Envelope = { t: "chunk"; value: unknown } | { t: "end" } | { t: "error"; error?: string };

interface StreamGlobal {
  __invokeStream: (streamId: number, cmd: string, argsJson: string) => void;
  __streamCancel: (streamId: number) => void;
  __streamPush?: (streamId: number, json: string) => void;
}

const streamGlobal = globalThis as unknown as StreamGlobal;

let nextStreamId = 1;
// Live streams keyed by id. Entries are removed on terminal (end/error) and on
// cancel, so a late `__streamPush` to a stale id is a harmless no-op.
const controllers = new Map<number, GluxeStream<unknown>>();

/** Called by Rust from the pump loop to push a chunk / close / error a stream. */
streamGlobal.__streamPush = (streamId: number, json: string): void => {
  const stream = controllers.get(streamId);
  if (!stream) return; // already cancelled or terminated
  const env = JSON.parse(json) as Envelope;
  if (env.t === "chunk") {
    stream._enqueue(env.value);
  } else if (env.t === "end") {
    controllers.delete(streamId);
    stream._close();
  } else {
    controllers.delete(streamId);
    stream._error(new Error(env.error ?? "stream failed"));
  }
};

const __invokeStream = streamGlobal.__invokeStream;
const __streamCancel = streamGlobal.__streamCancel;

interface Waiter<T> {
  resolve: (result: IteratorResult<T>) => void;
  reject: (error: unknown) => void;
}

/** A reader handle, mirroring `ReadableStreamDefaultReader`'s used surface. */
export interface GluxeStreamReader<T> {
  read(): Promise<IteratorResult<T>>;
  cancel(reason?: unknown): Promise<void>;
  releaseLock(): void;
}

/**
 * A minimal `ReadableStream`-compatible stream of `T` chunks produced by a
 * native streaming command. Consume it with `for await (const chunk of stream)`
 * or via `stream.getReader()`. `cancel()` (or breaking out of the loop) asks the
 * Rust handler to stop producing.
 */
export class GluxeStream<T> {
  // Buffered chunks awaiting a reader (push model — no backpressure to Rust, so
  // an unread fast stream buffers here unboundedly; consume promptly or cancel).
  private queue: T[] = [];
  // Pending `read()`/`next()` calls awaiting a chunk (one settler each).
  private waiters: Waiter<T>[] = [];
  private done = false;
  private err: unknown = null;
  private cancelled = false;

  constructor(private readonly streamId: number) {}

  /** @internal — push a chunk (called by `__streamPush`). */
  _enqueue(value: unknown): void {
    if (this.done || this.cancelled) return;
    const waiter = this.waiters.shift();
    if (waiter) {
      waiter.resolve({ value: value as T, done: false });
    } else {
      this.queue.push(value as T);
    }
  }

  /** @internal — graceful completion (called by `__streamPush`). */
  _close(): void {
    if (this.done) return;
    this.done = true;
    for (const w of this.waiters) {
      w.resolve({ value: undefined as never, done: true });
    }
    this.waiters = [];
  }

  /** @internal — error termination (called by `__streamPush`). */
  _error(error: unknown): void {
    if (this.done) return;
    this.done = true;
    this.err = error;
    for (const w of this.waiters) {
      w.reject(error);
    }
    this.waiters = [];
  }

  private pull(): Promise<IteratorResult<T>> {
    if (this.queue.length) {
      return Promise.resolve({ value: this.queue.shift() as T, done: false });
    }
    if (this.err) {
      const error = this.err;
      this.err = null; // surface the error once
      return Promise.reject(error);
    }
    if (this.done) {
      return Promise.resolve({ value: undefined as never, done: true });
    }
    return new Promise<IteratorResult<T>>((resolve, reject) => {
      this.waiters.push({ resolve, reject });
    });
  }

  /** Cooperatively cancel the stream: ask Rust to stop and close locally. */
  cancel(_reason?: unknown): void {
    if (this.cancelled || this.done) return;
    this.cancelled = true;
    controllers.delete(this.streamId);
    __streamCancel(this.streamId);
    this._close();
  }

  getReader(): GluxeStreamReader<T> {
    return {
      read: () => this.pull(),
      cancel: (reason?: unknown) => {
        this.cancel(reason);
        return Promise.resolve();
      },
      releaseLock: () => {},
    };
  }

  [Symbol.asyncIterator](): AsyncIterator<T> {
    return {
      next: () => this.pull(),
      // `for await` calls return() on break/throw — cancel the native side.
      return: (value?: unknown) => {
        this.cancel();
        return Promise.resolve({ value, done: true } as IteratorResult<T>);
      },
    };
  }
}

/**
 * Call a native **streaming** command and obtain a {@link GluxeStream} of its
 * chunks. The command must be registered with `PluginBuilder::stream_command`
 * on the Rust side; calling this on a sync/async command errors the stream.
 *
 * @param cmd Command key: `"{plugin}|{command}"`, e.g. `"fs|readFileStream"`.
 * @param args Serialised to JSON before being passed to Rust.
 */
export function invokeStream<T = unknown>(
  cmd: string,
  args: Record<string, unknown> = {},
): GluxeStream<T> {
  const id = nextStreamId++;
  const stream = new GluxeStream<T>(id);
  controllers.set(id, stream as GluxeStream<unknown>);
  __invokeStream(id, cmd, JSON.stringify(args));
  return stream;
}
