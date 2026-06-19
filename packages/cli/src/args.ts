// CLI argument validation: reject unknown options before gunshi parses them, so
// a typo like `--no-js` fails loudly instead of being silently ignored.

const commandOptions = {
  build: new Set(["help", "project", "release", "version"]),
  run: new Set(["help", "project", "release", "version"]),
  start: new Set(["help", "project", "version"]),
} as const;

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

function isCliCommand(command: string | undefined): command is keyof typeof commandOptions {
  return command === "build" || command === "run" || command === "start";
}
