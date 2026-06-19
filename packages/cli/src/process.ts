// Spawning child processes and tearing them down. `runCommand` runs a one-shot
// command to completion; `killTree` takes down a process and its descendants.

import { spawnSync } from "node:child_process";

import spawn from "cross-spawn";

export interface CommandSpec {
  command: string;
  args: string[];
  cwd: string;
}

// Kill a child and its descendants. Package-manager/cargo wrappers don't
// forward SIGTERM to grandchildren, so killing only the immediate child orphans
// them. On Windows, taskkill /T takes down the whole tree. On POSIX, children
// are spawned detached (own process-group leader) so we signal the whole group
// via negative pid, falling back to the lone child if the group is gone.
export function killTree(child: ReturnType<typeof spawn>): void {
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
