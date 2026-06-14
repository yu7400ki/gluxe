import { spawnSync } from "node:child_process";
import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

import spawn from "cross-spawn";

const BUNDLE_MANIFEST_FILE = "gluxe.manifest.json";

export interface BundleBuildConfig {
  command: string;
  args: string[];
  cwd: string;
}

export interface CommandSpec {
  command: string;
  args: string[];
  cwd: string;
}

const commandOptions = {
  build: new Set(["help", "project", "release", "version"]),
  run: new Set(["help", "project", "release", "version"]),
  start: new Set(["help", "project", "version"]),
} as const;

const defaultBundleArgs = ["run", "build"];

export function defaultBundleBuildConfig(project: string): BundleBuildConfig {
  return {
    command: "npm",
    args: [...defaultBundleArgs],
    cwd: project,
  };
}

async function readAppJson(project: string): Promise<Record<string, unknown>> {
  const appJsonPath = path.join(project, "app.json");

  let content: string;
  try {
    content = await readFile(appJsonPath, "utf8");
  } catch (error) {
    if (isNodeError(error) && error.code === "ENOENT") {
      return {};
    }
    throw error;
  }

  try {
    return getRecord(JSON.parse(content));
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`failed to parse ${appJsonPath}: ${message}`, { cause: error });
  }
}

function parseCommandSpec(
  record: Record<string, unknown>,
  project: string,
  fallback: BundleBuildConfig,
): BundleBuildConfig {
  const command = typeof record.command === "string" ? record.command : fallback.command;
  const args = Array.isArray(record.args)
    ? record.args.filter((value): value is string => typeof value === "string")
    : fallback.args;
  const cwd = typeof record.cwd === "string" ? path.resolve(project, record.cwd) : fallback.cwd;
  return { command, args, cwd };
}

export async function readBundleBuildConfig(project: string): Promise<BundleBuildConfig> {
  const json = await readAppJson(project);
  const build = getRecord(getRecord(json.bundle).build);
  return parseCommandSpec(build, project, defaultBundleBuildConfig(project));
}

export async function readBundleOutDir(project: string): Promise<string> {
  const json = await readAppJson(project);
  const outDir = getRecord(json.bundle).outDir;
  return typeof outDir === "string" ? outDir : "dist";
}

const defaultDevArgs = ["run", "dev"];

export function defaultDevBuildConfig(project: string): BundleBuildConfig {
  return {
    command: "npm",
    args: [...defaultDevArgs],
    cwd: project,
  };
}

// Watch-mode build command for `gluxe start`. Read from `dev.build` in
// app.json; defaults to the project's `dev` npm script. The CLI never
// constructs bundler-specific flags — the only contract is that the command
// keeps running and rewrites dist/ (manifest + entry) on every change.
export async function readDevBuildConfig(project: string): Promise<BundleBuildConfig> {
  const json = await readAppJson(project);
  const build = getRecord(getRecord(json.dev).build);
  return parseCommandSpec(build, project, defaultDevBuildConfig(project));
}

export async function resolveProject(project: string): Promise<string> {
  const resolved = path.resolve(project);
  try {
    await stat(resolved);
  } catch {
    throw new Error(`project path '${project}' not found`);
  }
  return resolved;
}

export async function ensureCargoToml(project: string): Promise<void> {
  try {
    const cargoToml = await stat(path.join(project, "Cargo.toml"));
    if (!cargoToml.isFile()) {
      throw new Error();
    }
  } catch {
    throw new Error(`No Cargo.toml found in '${project}'. Make sure this is a gluxe project.`);
  }
}

export function createCargoCommand(
  subcommand: "build" | "run",
  project: string,
  release: boolean,
): CommandSpec {
  return {
    command: "cargo",
    args: release ? [subcommand, "--release"] : [subcommand],
    cwd: project,
  };
}

export function validateKnownOptions(argv: readonly string[]): void {
  const command = argv.find((arg) => !arg.startsWith("-"));
  if (!isCliCommand(command)) {
    return;
  }

  const allowed = commandOptions[command];
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (!arg.startsWith("-")) {
      continue;
    }

    if (arg === "-h" || arg === "-v") {
      continue;
    }

    if (!arg.startsWith("--")) {
      throw new Error(`unexpected option '${arg}'`);
    }

    const option = arg.slice(2).split("=", 1)[0];
    if (!allowed.has(option)) {
      throw new Error(`unexpected option '--${option}'`);
    }

    if (
      option === "project" &&
      !arg.includes("=") &&
      argv[index + 1] &&
      !argv[index + 1].startsWith("-")
    ) {
      index += 1;
    }
  }
}

export async function buildProject(projectArg: string, release: boolean): Promise<void> {
  const project = await resolveProject(projectArg);
  await ensureCargoToml(project);
  await runBundleBuild(project);
  await runCommand(createCargoCommand("build", project, release), "`cargo build`");
}

export async function runProject(projectArg: string, release: boolean): Promise<void> {
  const project = await resolveProject(projectArg);
  await ensureCargoToml(project);
  await runBundleBuild(project);
  await runCommand(createCargoCommand("run", project, release), "`cargo run`");
}

export async function runBundleBuild(project: string): Promise<void> {
  const config = await readBundleBuildConfig(project);
  const suffix = config.args.length > 0 ? ` ${config.args.join(" ")}` : "";
  console.log(`-> Running JS bundle build: ${config.command}${suffix}`);
  await runCommand(config, "JS bundle build command");
}

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

// Kill a child and its descendants. Package-manager/cargo wrappers don't
// forward SIGTERM to grandchildren, so killing only the immediate child orphans
// them. On Windows, taskkill /T takes down the whole tree. On POSIX, children
// are spawned detached (own process-group leader) so we signal the whole group
// via negative pid, falling back to the lone child if the group is gone.
function killTree(child: ReturnType<typeof spawn>): void {
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

function getRecord(value: unknown): Record<string, unknown> {
  return value !== null && typeof value === "object" ? (value as Record<string, unknown>) : {};
}

function isNodeError(error: unknown): error is NodeJS.ErrnoException {
  return error instanceof Error && "code" in error;
}

function isCliCommand(command: string | undefined): command is keyof typeof commandOptions {
  return command === "build" || command === "run" || command === "start";
}
