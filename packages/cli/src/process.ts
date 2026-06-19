// Spawning child processes and tearing them down. `runCommand` runs a one-shot
// command to completion; `killTree` takes down a process and its descendants.

import { spawnSync } from "node:child_process";

import spawn from "cross-spawn";

export interface CommandSpec {
  command: string;
  args: string[];
  cwd: string;
}

/** A spawned child process (cross-spawn's return type). */
export type Child = ReturnType<typeof spawn>;

/** The spawn function, injectable into {@link ProcessSupervisor} for tests. */
export type Spawn = typeof spawn;

// Kill a child and its descendants. Package-manager/cargo wrappers don't
// forward SIGTERM to grandchildren, so killing only the immediate child orphans
// them. On Windows, taskkill /T takes down the whole tree. On POSIX, children
// are spawned detached (own process-group leader) so we signal the whole group
// via negative pid, falling back to the lone child if the group is gone.
export function killTree(child: Child): void {
  const pid = child.pid;
  if (pid === undefined || child.exitCode !== null || child.signalCode !== null) {
    return;
  }
  if (process.platform === "win32") {
    spawnSync("taskkill", ["/pid", String(pid), "/T", "/F"], { stdio: "ignore" });
  } else {
    try {
      process.kill(-pid, "SIGTERM");
    } catch {
      child.kill("SIGTERM");
    }
  }
}

export function runCommand(spec: CommandSpec, label: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const child = spawn(spec.command, spec.args, {
      cwd: spec.cwd,
      stdio: "inherit",
    });

    child.on("error", (error) => {
      reject(new Error(`failed to run ${label}: ${error.message}`));
    });
    child.on("exit", (code, signal) => {
      if (code === 0) {
        resolve();
        return;
      }
      if (signal) {
        reject(new Error(`${label} exited with signal ${signal}`));
        return;
      }
      reject(new Error(`${label} exited with status ${code}`));
    });
  });
}

/** Install SIGINT/SIGTERM handlers running `handler`; returns a remover. The
 *  remover form (rather than a scoped wrapper) lets the caller drop the handlers
 *  on an arbitrary settle point — e.g. when a watched child exits. */
export function onSignals(handler: () => void): () => void {
  process.on("SIGINT", handler);
  process.on("SIGTERM", handler);
  return () => {
    process.off("SIGINT", handler);
    process.off("SIGTERM", handler);
  };
}

/**
 * Tracks spawned children so the whole set is torn down in one place. Both
 * `gluxe start` lifecycle phases call {@link ProcessSupervisor.killAll}; since
 * {@link killTree} no-ops on an already-dead or unspawned child, killing the
 * full set is equivalent to the targeted per-child kills it replaces. `spawn`
 * and `kill` are injectable so the lifecycle can be tested with fake children.
 */
export class ProcessSupervisor {
  private readonly children: Child[] = [];

  constructor(
    private readonly spawnFn: Spawn = spawn,
    private readonly kill: (child: Child) => void = killTree,
  ) {}

  spawn(...args: Parameters<Spawn>): Child {
    const child = this.spawnFn(...args);
    this.children.push(child);
    return child;
  }

  killAll(): void {
    for (const child of this.children) {
      this.kill(child);
    }
  }
}
