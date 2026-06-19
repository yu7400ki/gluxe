// Dev mode (`gluxe start`): run the JS watch build, wait for its first bundle,
// then launch the debug binary with GLUXE_DEV_DIST so the runtime hot-reloads.

import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

import spawn from "cross-spawn";

import { BUNDLE_MANIFEST_FILE, getRecord, readBundleOutDir, readDevBuildConfig } from "./config.js";
import { killTree } from "./process.js";
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

  let entry: unknown;
  try {
    entry = getRecord(JSON.parse(await readFile(manifestPath, "utf8"))).entry;
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

  const suffix = devConfig.args.length > 0 ? ` ${devConfig.args.join(" ")}` : "";
  console.log(`-> Starting JS watch build: ${devConfig.command}${suffix}`);
  const watcher = spawn(devConfig.command, devConfig.args, {
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

  // Signal handler during the first-build wait: kill the detached watcher
  // before exiting (full app+watcher handling is installed after the app spawns).
  const onFirstBuildSignal = () => {
    killTree(watcher);
    process.exit(1);
  };
  process.on("SIGINT", onFirstBuildSignal);
  process.on("SIGTERM", onFirstBuildSignal);

  try {
    await waitForFirstBuild(dist, prevManifestMtime, () => watcherDeath);
  } catch (error) {
    process.off("SIGINT", onFirstBuildSignal);
    process.off("SIGTERM", onFirstBuildSignal);
    killTree(watcher);
    throw error;
  }
  process.off("SIGINT", onFirstBuildSignal);
  process.off("SIGTERM", onFirstBuildSignal);

  console.log(`-> Launching app (debug build) with GLUXE_DEV_DIST=${dist}`);
  const app = spawn("cargo", ["run"], {
    cwd: project,
    stdio: "inherit",
    detached,
    env: { ...process.env, GLUXE_DEV_DIST: dist },
  });

  return new Promise<void>((resolve, reject) => {
    let settled = false;
    let shuttingDown = false;

    const onSignal = () => {
      shuttingDown = true;
      killTree(app);
      killTree(watcher);
    };
    process.on("SIGINT", onSignal);
    process.on("SIGTERM", onSignal);

    const finish = (error?: Error) => {
      if (settled) {
        return;
      }
      settled = true;
      process.off("SIGINT", onSignal);
      process.off("SIGTERM", onSignal);
      if (error) {
        reject(error);
      } else {
        resolve();
      }
    };

    app.on("error", (error) => {
      killTree(watcher);
      finish(new Error(`failed to run \`cargo run\`: ${error.message}`));
    });
    app.on("exit", (code, signal) => {
      killTree(watcher);
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
        killTree(app);
        finish(new Error(`JS watch build exited unexpectedly (${watcherDeath})`));
      }
    });
  });
}
