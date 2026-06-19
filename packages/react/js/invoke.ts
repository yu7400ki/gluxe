// Native command bridge. `__invoke` is fire-and-forget: we hand Rust a monotonic call id,
// park the Promise settlers in `pending`, and Rust settles via `__resolveInvoke(id, json)`
// from the pump loop once the command result is ready.

import { createIdGenerator, hostGlobal, type InvokeEnvelope } from "./bridge-channel";

const nextId = createIdGenerator();
// Test seam — exported for characterization tests only; not part of the public API.
export const pending = new Map<
  number,
  { resolve: (value: unknown) => void; reject: (error: unknown) => void }
>();

/** Called by Rust from the pump loop to settle a parked Promise. */
hostGlobal.__resolveInvoke = (id: number, json: string): void => {
  const entry = pending.get(id);
  if (!entry) return;
  pending.delete(id);
  const response = JSON.parse(json) as InvokeEnvelope<unknown>;
  if (response.ok) {
    entry.resolve(response.value);
  } else {
    entry.reject(new Error(response.error ?? "command failed"));
  }
};

/**
 * Call a native command registered via the gluxe plugin system.
 *
 * @param cmd Command key: `"{plugin}|{command}"`, e.g. `"fs|readTextFile"`.
 * @param args Serialised to JSON before being passed to Rust.
 */
export function invoke<T = unknown>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const id = nextId();
  return new Promise<T>((resolve, reject) => {
    pending.set(id, { resolve: resolve as (v: unknown) => void, reject });
    hostGlobal.__invoke(id, cmd, JSON.stringify(args));
  });
}
