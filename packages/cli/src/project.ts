// Locating and validating the app project on disk, and building the cargo
// command that compiles it.

import { stat } from "node:fs/promises";
import path from "node:path";

import type { CommandSpec } from "./process.js";

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
