#!/usr/bin/env node
import { cli, define } from "gunshi";

import packageJson from "../package.json" with { type: "json" };
import { validateKnownOptions } from "./args.js";
import { buildProject, runProject } from "./commands.js";
import { startProject } from "./dev.js";

const projectArgs = {
  project: {
    type: "string",
    default: ".",
    description: "Root of the project",
  },
  release: {
    type: "boolean",
    description: "Build in release mode",
  },
} as const;

const mainCommand = define({
  name: "gluxe",
  description: "gluxe toolchain - build and run gluxe apps",
  run: () => {
    console.log("Run `gluxe --help` for available commands.");
  },
});

const buildCommand = define({
  name: "build",
  description: "Build the project",
  args: projectArgs,
  run: async (ctx) => {
    await buildProject(ctx.values.project ?? ".", Boolean(ctx.values.release));
  },
});

const runCommand = define({
  name: "run",
  description: "Build and run the project",
  args: projectArgs,
  run: async (ctx) => {
    await runProject(ctx.values.project ?? ".", Boolean(ctx.values.release));
  },
});

const startCommand = define({
  name: "start",
  description: "Start the dev server with live reload",
  args: {
    project: {
      type: "string",
      default: ".",
      description: "Root of the project",
    },
  },
  run: async (ctx) => {
    await startProject(ctx.values.project ?? ".");
  },
});

try {
  const argv = process.argv.slice(2);
  validateKnownOptions(argv);

  await cli(argv, mainCommand, {
    name: "gluxe",
    version: packageJson.version,
    subCommands: {
      build: buildCommand,
      run: runCommand,
      start: startCommand,
    },
  });
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
