// Reading the app project's configuration: the bundle/dev build commands and
// output dir from app.json, plus the small JSON helpers they share.

import { readFile } from "node:fs/promises";
import path from "node:path";

export const BUNDLE_MANIFEST_FILE = "gluxe.manifest.json";

export interface BundleBuildConfig {
  command: string;
  args: string[];
  cwd: string;
}

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

export function getRecord(value: unknown): Record<string, unknown> {
  return value !== null && typeof value === "object" ? (value as Record<string, unknown>) : {};
}

function isNodeError(error: unknown): error is NodeJS.ErrnoException {
  return error instanceof Error && "code" in error;
}
