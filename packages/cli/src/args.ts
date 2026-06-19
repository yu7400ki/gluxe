// CLI argument schemas + validation. The schemas are the single source of truth:
// index.ts builds the gunshi commands from them, and validateKnownOptions derives
// its per-command allowlist from them, so the parser and the validator can't drift.

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

const startArgs = {
  project: {
    type: "string",
    default: ".",
    description: "Root of the project",
  },
} as const;

/** Per-command gunshi argument schemas, shared with index.ts. */
export const commandArgs = {
  build: projectArgs,
  run: projectArgs,
  start: startArgs,
} as const;

// gunshi adds these to every command; they are not part of the schemas above.
const builtinFlags = ["help", "version"];

export function validateKnownOptions(argv: readonly string[]): void {
  const command = argv.find((arg) => !arg.startsWith("-"));
  if (!isCliCommand(command)) {
    return;
  }

  const allowed = new Set<string>([...Object.keys(commandArgs[command]), ...builtinFlags]);
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

function isCliCommand(command: string | undefined): command is keyof typeof commandArgs {
  return command !== undefined && Object.hasOwn(commandArgs, command);
}
