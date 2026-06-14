// Native command bridge. `__invoke` is fire-and-forget: we hand Rust a monotonic call id,
// park the Promise settlers in `pending`, and Rust settles via `__resolveInvoke(id, json)`
// from the pump loop once the command result is ready.

interface InvokeResponse<T> {
  ok: boolean;
  value?: T;
  error?: string;
}

type NativeInvoke = (id: number, cmd: string, argsJson: string) => void;

interface InvokeGlobal {
  __resolveInvoke?: (id: number, json: string) => void;
  __invoke: NativeInvoke;
}

const invokeGlobal = globalThis as unknown as InvokeGlobal;

let nextId = 1;
const pending = new Map<
  number,
  { resolve: (value: unknown) => void; reject: (error: unknown) => void }
>();

/** Called by Rust from the pump loop to settle a parked Promise. */
invokeGlobal.__resolveInvoke = (id: number, json: string): void => {
  const entry = pending.get(id);
  if (!entry) return;
  pending.delete(id);
  const response = JSON.parse(json) as InvokeResponse<unknown>;
  if (response.ok) {
    entry.resolve(response.value);
  } else {
    entry.reject(new Error(response.error ?? "command failed"));
  }
};

const __invoke = invokeGlobal.__invoke;

/**
 * Call a native command registered via the gluxe plugin system.
 *
 * @param cmd Command key: `"{plugin}|{command}"`, e.g. `"fs|readTextFile"`.
 * @param args Serialised to JSON before being passed to Rust.
 */
export function invoke<T = unknown>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const id = nextId++;
  return new Promise<T>((resolve, reject) => {
    pending.set(id, { resolve: resolve as (v: unknown) => void, reject });
    __invoke(id, cmd, JSON.stringify(args));
  });
}
