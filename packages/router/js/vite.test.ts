import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { afterEach, describe, expect, it, vi } from "vitest";

import { gluxeRouter } from "./vite";

const VIRTUAL_ID = "virtual:@gluxe/router/routes";
const RESOLVED_ID = `\0${VIRTUAL_ID}`;

const tempDirs: string[] = [];

async function createProject(files: Record<string, string>): Promise<string> {
  const root = await mkdtemp(path.join(os.tmpdir(), "rg-router-test-"));
  tempDirs.push(root);
  for (const [relPath, content] of Object.entries(files)) {
    const abs = path.join(root, relPath);
    await mkdir(path.dirname(abs), { recursive: true });
    await writeFile(abs, content);
  }
  return root;
}

afterEach(async () => {
  await Promise.all(tempDirs.splice(0).map((dir) => rm(dir, { recursive: true, force: true })));
});

const PAGE = "export default function Page() { return null; }\n";

interface LoadResult {
  code: string;
  watched: string[];
}

/** Drive the plugin like Vite would: configResolved → resolveId → load. */
function loadVirtualModule(root: string, options?: { routesDir?: string }): LoadResult {
  const plugin = gluxeRouter(options);
  (plugin.configResolved as (config: { root: string }) => void)({ root });
  const resolved = (plugin.resolveId as (id: string) => string | undefined)(VIRTUAL_ID);
  expect(resolved).toBe(RESOLVED_ID);
  const addWatchFile = vi.fn();
  const code = (plugin.load as (this: unknown, id: string) => string | undefined).call(
    { addWatchFile },
    RESOLVED_ID,
  );
  if (typeof code !== "string") throw new Error("load() returned no code");
  return { code, watched: addWatchFile.mock.calls.map((call) => call[0] as string) };
}

/**
 * Evaluate the generated module with imports stubbed out, so each route's
 * `component` becomes the (forward-slash) absolute path it was imported from.
 */
function evalRoutes(code: string): unknown {
  const body = code
    .replace(/^import (\w+) from ("[^"]+");$/gm, "const $1 = $2;")
    .replace("export const routes", "const routes");
  expect(body).not.toContain("import "); // static imports only, all rewritten
  return new Function(`${body}\nreturn routes;`)();
}

/** Forward-slash absolute path of a route file, as emitted by the codegen. */
function file(root: string, relPath: string): string {
  return path.join(root, "src/routes", relPath).replace(/\\/g, "/");
}

describe("gluxeRouter", () => {
  it("leaves unrelated ids unresolved", async () => {
    const plugin = gluxeRouter();
    const resolveId = plugin.resolveId as (id: string) => string | undefined;
    expect(resolveId("react")).toBeUndefined();
    expect(resolveId("./routes")).toBeUndefined();
  });

  it("builds the full route tree from pages-style conventions", async () => {
    const root = await createProject({
      "src/routes/_layout.tsx": PAGE,
      "src/routes/index.tsx": PAGE,
      "src/routes/about.tsx": PAGE,
      "src/routes/404.tsx": PAGE,
      "src/routes/users/index.tsx": PAGE,
      "src/routes/users/[id].tsx": PAGE,
    });
    const { code } = loadVirtualModule(root);
    expect(evalRoutes(code)).toEqual([
      {
        id: "/_layout",
        component: file(root, "_layout.tsx"),
        children: [
          { id: "/*", path: "*", component: file(root, "404.tsx") },
          { id: "/about", path: "about", component: file(root, "about.tsx") },
          { id: "/", index: true, component: file(root, "index.tsx") },
          {
            id: "/users",
            path: "users",
            children: [
              { id: "/users/:id", path: ":id", component: file(root, "users/[id].tsx") },
              { id: "/users/", index: true, component: file(root, "users/index.tsx") },
            ],
          },
        ],
      },
    ]);
  });

  it("emits a flat array when there is no root _layout", async () => {
    const root = await createProject({
      "src/routes/index.tsx": PAGE,
      "src/routes/about.tsx": PAGE,
    });
    expect(evalRoutes(loadVirtualModule(root).code)).toEqual([
      { id: "/about", path: "about", component: file(root, "about.tsx") },
      { id: "/", index: true, component: file(root, "index.tsx") },
    ]);
  });

  it("makes a directory without _layout a component-less pass-through node", async () => {
    const root = await createProject({
      "src/routes/settings/profile.tsx": PAGE,
    });
    expect(evalRoutes(loadVirtualModule(root).code)).toEqual([
      {
        id: "/settings",
        path: "settings",
        children: [
          {
            id: "/settings/profile",
            path: "profile",
            component: file(root, "settings/profile.tsx"),
          },
        ],
      },
    ]);
  });

  it("gives a nested directory its _layout as the level component", async () => {
    const root = await createProject({
      "src/routes/users/_layout.tsx": PAGE,
      "src/routes/users/index.tsx": PAGE,
    });
    expect(evalRoutes(loadVirtualModule(root).code)).toEqual([
      {
        id: "/users",
        path: "users",
        component: file(root, "users/_layout.tsx"),
        children: [{ id: "/users/", index: true, component: file(root, "users/index.tsx") }],
      },
    ]);
  });

  it("converts [param] file and directory names to :param segments", async () => {
    const root = await createProject({
      "src/routes/[lang]/about.tsx": PAGE,
    });
    expect(evalRoutes(loadVirtualModule(root).code)).toEqual([
      {
        id: "/:lang",
        path: ":lang",
        children: [
          { id: "/:lang/about", path: "about", component: file(root, "[lang]/about.tsx") },
        ],
      },
    ]);
  });

  it("ignores underscore-prefixed files (except _layout), dotfiles, _dirs, and non-route extensions", async () => {
    const root = await createProject({
      "src/routes/index.tsx": PAGE,
      "src/routes/_helpers.tsx": PAGE,
      "src/routes/.hidden.tsx": PAGE,
      "src/routes/styles.css": "",
      "src/routes/_components/button.tsx": PAGE,
    });
    expect(evalRoutes(loadVirtualModule(root).code)).toEqual([
      { id: "/", index: true, component: file(root, "index.tsx") },
    ]);
  });

  it("skips directories that only contain a _layout (a layout never matches alone)", async () => {
    const root = await createProject({
      "src/routes/index.tsx": PAGE,
      "src/routes/admin/_layout.tsx": PAGE,
    });
    expect(evalRoutes(loadVirtualModule(root).code)).toEqual([
      { id: "/", index: true, component: file(root, "index.tsx") },
    ]);
  });

  it("skips directories that contain no route files", async () => {
    const root = await createProject({
      "src/routes/index.tsx": PAGE,
      "src/routes/empty/notes.txt": "",
    });
    expect(evalRoutes(loadVirtualModule(root).code)).toEqual([
      { id: "/", index: true, component: file(root, "index.tsx") },
    ]);
  });

  it("supports .ts/.jsx/.js route files", async () => {
    const root = await createProject({
      "src/routes/a.ts": PAGE,
      "src/routes/b.jsx": PAGE,
      "src/routes/c.js": PAGE,
    });
    expect(evalRoutes(loadVirtualModule(root).code)).toEqual([
      { id: "/a", path: "a", component: file(root, "a.ts") },
      { id: "/b", path: "b", component: file(root, "b.jsx") },
      { id: "/c", path: "c", component: file(root, "c.js") },
    ]);
  });

  it("respects the routesDir option", async () => {
    const root = await createProject({
      "pages/index.tsx": PAGE,
    });
    const { code } = loadVirtualModule(root, { routesDir: "pages" });
    expect(evalRoutes(code)).toEqual([
      { id: "/", index: true, component: path.join(root, "pages/index.tsx").replace(/\\/g, "/") },
    ]);
  });

  it("watches the routes directory and every route file", async () => {
    const root = await createProject({
      "src/routes/index.tsx": PAGE,
      "src/routes/users/[id].tsx": PAGE,
    });
    const { watched } = loadVirtualModule(root);
    expect(watched).toContain(path.resolve(root, "src/routes"));
    expect(watched).toContain(path.join(root, "src/routes", "index.tsx"));
    expect(watched).toContain(path.join(root, "src/routes", "users", "[id].tsx"));
  });

  it("emits Windows-safe import specifiers (no backslashes)", async () => {
    const root = await createProject({
      "src/routes/index.tsx": PAGE,
    });
    const { code } = loadVirtualModule(root);
    expect(code).not.toContain("\\");
  });

  it("throws when the routes directory is missing", async () => {
    const root = await createProject({});
    expect(() => loadVirtualModule(root)).toThrow(/Routes directory not found/);
  });

  it("throws when a file and a directory map to the same route", async () => {
    const root = await createProject({
      "src/routes/users.tsx": PAGE,
      "src/routes/users/index.tsx": PAGE,
    });
    expect(() => loadVirtualModule(root)).toThrow(/coexist/);
  });

  it("throws when two files map to the same route", async () => {
    const root = await createProject({
      "src/routes/about.tsx": PAGE,
      "src/routes/about.ts": PAGE,
    });
    expect(() => loadVirtualModule(root)).toThrow(/Conflicting route files/);
  });

  it("returns nothing for other module ids", async () => {
    const root = await createProject({ "src/routes/index.tsx": PAGE });
    const plugin = gluxeRouter();
    (plugin.configResolved as (config: { root: string }) => void)({ root });
    const load = plugin.load as (this: unknown, id: string) => string | undefined;
    expect(load.call({ addWatchFile: vi.fn() }, "/some/other/file.ts")).toBeUndefined();
  });
});
