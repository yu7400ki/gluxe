import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { validateKnownOptions } from "./args.js";
import { readBundleBuildConfig, readBundleOutDir, readDevBuildConfig } from "./config.js";
import { checkManifestReady, manifestMtime, waitForFirstBuild } from "./dev.js";
import { createCargoCommand, ensureCargoToml } from "./project.js";

describe("readBundleBuildConfig", () => {
  it("uses the default command when app.json is missing", async () => {
    const project = await createTempProject();
    try {
      await expect(readBundleBuildConfig(project)).resolves.toEqual({
        command: "npm",
        args: ["run", "build"],
        cwd: project,
      });
    } finally {
      await removeTempProject(project);
    }
  });

  it("uses the default command when bundle.build is missing", async () => {
    const project = await createTempProject({
      "app.json": JSON.stringify({ name: "app" }),
    });
    try {
      await expect(readBundleBuildConfig(project)).resolves.toEqual({
        command: "npm",
        args: ["run", "build"],
        cwd: project,
      });
    } finally {
      await removeTempProject(project);
    }
  });

  it("reads custom build command, args, and cwd", async () => {
    const project = await createTempProject({
      "app.json": JSON.stringify({
        bundle: {
          build: {
            command: "npm",
            args: ["run", "bundle"],
            cwd: "web",
          },
        },
      }),
    });
    try {
      await expect(readBundleBuildConfig(project)).resolves.toEqual({
        command: "npm",
        args: ["run", "bundle"],
        cwd: path.join(project, "web"),
      });
    } finally {
      await removeTempProject(project);
    }
  });

  it("reports JSON parse errors with the app.json path", async () => {
    const project = await createTempProject({
      "app.json": "{",
    });
    try {
      await expect(readBundleBuildConfig(project)).rejects.toThrow(
        `failed to parse ${path.join(project, "app.json")}`,
      );
    } finally {
      await removeTempProject(project);
    }
  });
});

describe("ensureCargoToml", () => {
  it("rejects projects without Cargo.toml", async () => {
    const project = await createTempProject();
    try {
      await expect(ensureCargoToml(project)).rejects.toThrow("No Cargo.toml found");
    } finally {
      await removeTempProject(project);
    }
  });

  it("accepts projects with Cargo.toml", async () => {
    const project = await createTempProject({
      "Cargo.toml": '[package]\nname = "example"\n',
    });
    try {
      await expect(ensureCargoToml(project)).resolves.toBeUndefined();
    } finally {
      await removeTempProject(project);
    }
  });
});

describe("createCargoCommand", () => {
  it("creates build and run commands", () => {
    expect(createCargoCommand("build", "app", false)).toEqual({
      command: "cargo",
      args: ["build"],
      cwd: "app",
    });
    expect(createCargoCommand("run", "app", false)).toEqual({
      command: "cargo",
      args: ["run"],
      cwd: "app",
    });
  });

  it("adds --release when requested", () => {
    expect(createCargoCommand("build", "app", true)).toEqual({
      command: "cargo",
      args: ["build", "--release"],
      cwd: "app",
    });
  });
});

describe("readDevBuildConfig", () => {
  it("defaults to `npm run dev` when app.json is missing", async () => {
    const project = await createTempProject();
    try {
      await expect(readDevBuildConfig(project)).resolves.toEqual({
        command: "npm",
        args: ["run", "dev"],
        cwd: project,
      });
    } finally {
      await removeTempProject(project);
    }
  });

  it("defaults to `npm run dev` when dev.build is missing (bundle.build is unrelated)", async () => {
    const project = await createTempProject({
      "app.json": JSON.stringify({
        bundle: { build: { command: "pnpm", args: ["build"] } },
      }),
    });
    try {
      await expect(readDevBuildConfig(project)).resolves.toEqual({
        command: "npm",
        args: ["run", "dev"],
        cwd: project,
      });
    } finally {
      await removeTempProject(project);
    }
  });

  it("uses dev.build verbatim when present", async () => {
    const project = await createTempProject({
      "app.json": JSON.stringify({
        bundle: { build: { command: "pnpm", args: ["build"] } },
        dev: { build: { command: "pnpm", args: ["watch"], cwd: "web" } },
      }),
    });
    try {
      await expect(readDevBuildConfig(project)).resolves.toEqual({
        command: "pnpm",
        args: ["watch"],
        cwd: path.join(project, "web"),
      });
    } finally {
      await removeTempProject(project);
    }
  });
});

describe("readBundleOutDir", () => {
  it("defaults to dist", async () => {
    const project = await createTempProject();
    try {
      await expect(readBundleOutDir(project)).resolves.toBe("dist");
    } finally {
      await removeTempProject(project);
    }
  });

  it("reads bundle.outDir", async () => {
    const project = await createTempProject({
      "app.json": JSON.stringify({ bundle: { outDir: "out" } }),
    });
    try {
      await expect(readBundleOutDir(project)).resolves.toBe("out");
    } finally {
      await removeTempProject(project);
    }
  });
});

describe("checkManifestReady", () => {
  it("is false when the manifest is missing", async () => {
    const project = await createTempProject();
    try {
      await expect(checkManifestReady(project, null)).resolves.toBe(false);
    } finally {
      await removeTempProject(project);
    }
  });

  it("is false when the manifest mtime equals the pre-start snapshot", async () => {
    const project = await createTempProject({
      "gluxe.manifest.json": JSON.stringify({ version: 1, entry: "assets/index-abc.js" }),
      "assets/index-abc.js": "//",
    });
    try {
      const prev = await manifestMtime(project);
      await expect(checkManifestReady(project, prev)).resolves.toBe(false);
    } finally {
      await removeTempProject(project);
    }
  });

  it("is false when the entry file is missing", async () => {
    const project = await createTempProject({
      "gluxe.manifest.json": JSON.stringify({ version: 1, entry: "index-abc.js" }),
    });
    try {
      await expect(checkManifestReady(project, null)).resolves.toBe(false);
    } finally {
      await removeTempProject(project);
    }
  });

  it("is true for a fresh manifest whose entry exists", async () => {
    const project = await createTempProject({
      "gluxe.manifest.json": JSON.stringify({ version: 1, entry: "assets/index-abc.js" }),
      "assets/index-abc.js": "//",
    });
    try {
      await expect(checkManifestReady(project, null)).resolves.toBe(true);
    } finally {
      await removeTempProject(project);
    }
  });
});

describe("waitForFirstBuild", () => {
  it("throws an accurate error when the watcher dies after the bundle is ready", async () => {
    const project = await createTempProject({
      "dist/gluxe.manifest.json": JSON.stringify({ version: 1, entry: "index-abc.js" }),
      "dist/index-abc.js": "//",
    });
    try {
      const dist = path.join(project, "dist");
      // Watcher is already dead; manifest exists with no prior snapshot → counts as produced.
      await expect(waitForFirstBuild(dist, null, () => "status 0")).rejects.toThrow(
        "JS watch build exited after producing a bundle (status 0); `gluxe start` needs a watch-mode command that keeps running (check dev.build in app.json)",
      );
    } finally {
      await removeTempProject(project);
    }
  });

  it("throws 'before producing' when the watcher dies without writing a bundle", async () => {
    const project = await createTempProject();
    try {
      const dist = path.join(project, "dist");
      // No manifest at all; watcher died immediately.
      await expect(waitForFirstBuild(dist, null, () => "status 1")).rejects.toThrow(
        "JS watch build exited before producing a bundle (status 1); check that the dev command exists (dev.build in app.json, default: `npm run dev`)",
      );
    } finally {
      await removeTempProject(project);
    }
  });
});

describe("validateKnownOptions", () => {
  it("accepts supported command options", () => {
    expect(() =>
      validateKnownOptions(["build", "--project", "examples/counter", "--release"]),
    ).not.toThrow();
    expect(() => validateKnownOptions(["run", "--project=examples/counter"])).not.toThrow();
    expect(() => validateKnownOptions(["start", "--project", "examples/counter"])).not.toThrow();
  });

  it("rejects removed --no-js option", () => {
    expect(() => validateKnownOptions(["build", "--no-js"])).toThrow("unexpected option '--no-js'");
  });

  it("rejects unsupported short options", () => {
    expect(() => validateKnownOptions(["build", "-x"])).toThrow("unexpected option '-x'");
  });
});

async function createTempProject(files: Record<string, string> = {}): Promise<string> {
  const project = await mkdtemp(path.join(os.tmpdir(), "gluxe-cli-"));
  for (const [filename, content] of Object.entries(files)) {
    const filePath = path.join(project, filename);
    await mkdir(path.dirname(filePath), { recursive: true });
    await writeFile(filePath, content);
  }
  return project;
}

async function removeTempProject(project: string): Promise<void> {
  await rm(project, { recursive: true, force: true });
}
