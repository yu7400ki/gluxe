// Single source of truth for the JS↔Rust global channel.
//
// Rust installs the `__*` functions onto the Boa global at context init (see
// `bridge::register_all`); JS installs its own callbacks back (`__resolveInvoke`
// / `__streamPush` / `__dispatchEvent`), which Rust calls from the pump loop.
// Every module that touches these reaches them through `hostGlobal` here, so the
// `globalThis as unknown as …` cast — unavoidable because the globals are
// injected by the host rather than declared in TS — lives in exactly one place.

/** Reconciler bridge ops, installed by Rust as `globalThis.__bridge`. */
export interface Bridge {
  createInstance(type: string, props: Record<string, unknown>, events: string[]): number;
  createText(text: string): number;
  appendChild(parentId: number, childId: number): void;
  appendToContainer(childId: number): void;
  insertBefore(parentId: number, childId: number, beforeId: number): void;
  insertInContainer(childId: number, beforeId: number): void;
  removeChild(parentId: number, childId: number): void;
  removeFromContainer(childId: number): void;
  updateProps(id: number, props: Record<string, unknown>, events: string[], type: string): void;
  updateText(id: number, text: string): void;
  clearContainer(): void;
  detachDeleted(id: number): void;
  /** Focused element id (any kind) as of the last paint, or `null`. */
  getActiveElement(): number | null;
  /** Tab-stop focusable ids in a subtree, in Tab order. */
  getFocusableElements(rootId: number): number[];
  /** Confine Tab to a subtree (push onto the scope stack). */
  pushTabScope(rootId: number): void;
  /** Release a Tab scope by id. */
  popTabScope(rootId: number): void;
}

/**
 * The complete set of globals exchanged across the JS↔Rust channel.
 *
 * The Rust-installed entries are always present once the context is initialised.
 * The JS-installed callbacks are optional in the type because the module that
 * owns each one assigns it at import time.
 */
export interface HostGlobal {
  // ── Installed by Rust (bridge::register_all) ──
  __bridge: Bridge;
  /** `(callId, cmdKey, argsJson) → void`; the result is delivered later via `__resolveInvoke`. */
  __invoke: (callId: number, cmdKey: string, argsJson: string) => void;
  /** `(streamId, cmdKey, argsJson) → void`; chunks arrive later via `__streamPush`. */
  __invokeStream: (streamId: number, cmdKey: string, argsJson: string) => void;
  /** Cooperatively cancel a running stream (the Rust handler polls for it). */
  __streamCancel: (streamId: number) => void;
  // ── Installed by JS, called by Rust from the pump loop ──
  /** Settles a parked `invoke` Promise (installed by invoke.ts). */
  __resolveInvoke?: (callId: number, json: string) => void;
  /** Pushes a chunk / closes / errors a stream (installed by stream.ts). */
  __streamPush?: (streamId: number, json: string) => void;
  /** Routes a GPUI event to its registered JS handler (installed by host-config.ts). */
  __dispatchEvent?: (id: number, type: string, payload: Record<string, unknown>) => void;
}

/**
 * The injected host globals, typed. The cast is unavoidable (TS can't know about
 * host-injected globals) but is contained here so no other module repeats it.
 */
export const hostGlobal = globalThis as unknown as HostGlobal;

/**
 * A monotonic id generator for invoke call-ids / stream-ids. Starts at `1` (the
 * Rust side treats `0` as a "none" sentinel) and never repeats within a session,
 * so a late settle / push to an already-removed id is a harmless stale no-op.
 * Each caller gets its own independent counter.
 */
export function createIdGenerator(): () => number {
  let next = 1;
  return () => next++;
}

/**
 * Result envelope for a single `invoke` call — the JSON payload Rust passes to
 * `__resolveInvoke`. Mirrors the Rust `{"ok":true,"value":…}` /
 * `{"ok":false,"error":…}` shape.
 */
export interface InvokeEnvelope<T> {
  ok: boolean;
  value?: T;
  error?: string;
}

/**
 * One frame of a streaming command — the JSON payload Rust passes to
 * `__streamPush`. A `chunk` carries a value; `end` / `error` are terminal.
 */
export type StreamEnvelope =
  | { t: "chunk"; value: unknown }
  | { t: "end" }
  | { t: "error"; error?: string };
