// Dev mode (`gluxe start`): run the JS watch build, wait for its first bundle,
// then launch the debug binary with GLUXE_DEV_DIST so the runtime hot-reloads.

import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

import { BUNDLE_MANIFEST_FILE, type BundleManifest } from "@gluxe/config/manifest";

import { getRecord, readBundleOutDir, readDevBuildConfig } from "./config.js";
import { type Child, onSignals, ProcessSupervisor } from "./process.js";
import { ensureCargoToml, resolveProject } from "./project.js";

// Snapshot the manifest mtime before the watch build starts so stale output
// from a previous session is never mistaken for the first build's result.
export async function manifestMtime(dist: string): Promise<number | null> {
  try {
    return (await stat(path.join(dist, BUNDLE_MANIFEST_FILE))).mtimeMs;
  } catch {
    return null;
  }
}

// True when the manifest is newer than the pre-start snapshot and its entry
// file exists — i.e. the watch build has completed at least one fresh build.
export async function checkManifestReady(
  dist: string,
  prevMtimeMs: number | null,
): Promise<boolean> {
  const manifestPath = path.join(dist, BUNDLE_MANIFEST_FILE);
  let manifestStat;
  try {
    manifestStat = await stat(manifestPath);
  } catch {
    return false;
  }
  if (manifestStat.mtimeMs === prevMtimeMs) {
    return false; // still the previous session's build
  }

  let entry: BundleManifest["entry"] | undefined;
  try {
    entry = (getRecord(JSON.parse(await readFile(manifestPath, "utf8"))) as Partial<BundleManifest>)
      .entry;
  } catch {
    return false;
  }
  if (typeof entry !== "string") {
    return false;
  }

  try {
    return (await stat(path.join(dist, entry))).isFile();
  } catch {
    return false;
  }
}

const FIRST_BUILD_TIMEOUT_MS = 5 * 60 * 1000;

export async function waitForFirstBuild(
  dist: string,
  prevMtimeMs: number | null,
  watcherDied: () => string | null,
): Promise<void> {
  const start = Date.now();
  for (;;) {
    if (await checkManifestReady(dist, prevMtimeMs)) {
      const died = watcherDied();
      if (died !== null) {
        throw new Error(
          `JS watch build exited after producing a bundle (${died}); \`gluxe start\` needs a watch-mode command that keeps running (check dev.build in app.json)`,
        );
      }
      return;
    }
    const died = watcherDied();
    if (died !== null) {
      throw new Error(
        `JS watch build exited before producing a bundle (${died}); check that the dev command exists (dev.build in app.json, default: \`npm run dev\`)`,
      );
    }
    if (Date.now() - start > FIRST_BUILD_TIMEOUT_MS) {
      throw new Error(
        `timed out waiting for the first JS build (no fresh ${BUNDLE_MANIFEST_FILE} in ${dist})`,
      );
    }
    await delay(300);
  }
}

// `gluxe start` — dev mode: run the JS watch build, wait for the first build,
// then launch the debug binary with GLUXE_DEV_DIST=dist/ so the runtime
// hot-reloads on change. Either process dying (or Ctrl+C) kills the other.
export async function startProject(projectArg: string): Promise<void> {
  const project = await resolveProject(projectArg);
  await ensureCargoToml(project);

  const devConfig = await readDevBuildConfig(project);
  // bundle.outDir is relative to the build cwd, not necessarily the project root.
  const dist = path.resolve(devConfig.cwd, await readBundleOutDir(project));
  const prevManifestMtime = await manifestMtime(dist);

  // On POSIX, detached makes each child its own process-group leader (required
  // for killTree to signal the whole tree). It also detaches them from our
  // foreground group, so Ctrl+C no longer reaches them — we forward signals
  // manually. On Windows, taskkill /T handles the tree without detaching.
  const detached = process.platform !== "win32";

  // One supervisor owns both children; killAll() is the single teardown shared
  // by the first-build wait and the running-app supervision below.
  const supervisor = new ProcessSupervisor();

  const suffix = devConfig.args.length > 0 ? ` ${devConfig.args.join(" ")}` : "";
  console.log(`-> Starting JS watch build: ${devConfig.command}${suffix}`);
  const watcher = supervisor.spawn(devConfig.command, devConfig.args, {
    cwd: devConfig.cwd,
    stdio: "inherit",
    detached,
  });

  let watcherDeath: string | null = null;
  watcher.on("exit", (code, signal) => {
    watcherDeath = signal ? `signal ${signal}` : `status ${code}`;
  });
  watcher.on("error", (error) => {
    watcherDeath = error.message;
  });

  // During the first-build wait, a signal just tears the watcher down and exits
  // (full app+watcher supervision is installed once the app spawns).
  const removeFirstBuildSignals = onSignals(() => {
    supervisor.killAll();
    process.exit(1);
  });
  try {
    await waitForFirstBuild(dist, prevManifestMtime, () => watcherDeath);
  } catch (error) {
    supervisor.killAll();
    throw error;
  } finally {
    removeFirstBuildSignals();
  }

  console.log(`-> Launching app (debug build) with GLUXE_DEV_DIST=${dist}`);
  const app = supervisor.spawn("cargo", ["run"], {
    cwd: project,
    stdio: "inherit",
    detached,
    env: { ...process.env, GLUXE_DEV_DIST: dist },
  });

  return superviseApp(
    app,
    watcher,
    () => watcherDeath,
    () => supervisor.killAll(),
  );
}

/**
 * Supervise the running app against the watch build: resolve when the app exits
 * cleanly (or once we initiated shutdown), reject if it crashes or if the watch
 * build — which should outlive it — dies first. `killAll` tears down both
 * children (see {@link ProcessSupervisor}); it is safe to call on every path
 * because killTree no-ops on already-dead children. A SIGINT/SIGTERM tears both
 * down and resolves. Pure of any spawning, so it is unit-tested with fake
 * children.
 */
export function superviseApp(
  app: Child,
  watcher: Child,
  watcherDeath: () => string | null,
  killAll: () => void,
): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    let settled = false;
    let shuttingDown = false;

    const removeSignals = onSignals(() => {
      shuttingDown = true;
      killAll();
    });

    const finish = (error?: Error) => {
      if (settled) {
        return;
      }
      settled = true;
      removeSignals();
      if (error) {
        reject(error);
      } else {
        resolve();
      }
    };

    app.on("error", (error) => {
      killAll();
      finish(new Error(`failed to run \`cargo run\`: ${error.message}`));
    });
    app.on("exit", (code, signal) => {
      killAll();
      if (code === 0 || shuttingDown) {
        finish();
      } else if (signal) {
        finish(new Error(`\`cargo run\` exited with signal ${signal}`));
      } else {
        finish(new Error(`\`cargo run\` exited with status ${code}`));
      }
    });
    watcher.on("exit", () => {
      // The watch build should outlive the app; dying first is an error unless we initiated shutdown.
      if (!settled && !shuttingDown) {
        killAll();
        finish(new Error(`JS watch build exited unexpectedly (${watcherDeath()})`));
      }
    });
  });
}
