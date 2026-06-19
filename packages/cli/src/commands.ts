// The `gluxe build` / `gluxe run` command entry points: bundle the JS, then
// compile (and, for run, launch) the native binary. (`gluxe start` lives in
// dev.ts.)

import { readBundleBuildConfig } from "./config.js";
import { runCommand } from "./process.js";
import { createCargoCommand, ensureCargoToml, resolveProject } from "./project.js";

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
