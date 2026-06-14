import fs from "node:fs";
import path from "node:path";

import type { Plugin } from "vite";

const BUNDLE_MANIFEST_FILE = "gluxe.manifest.json";

export type GluxeOptions = {
  configPath?: string; // default: "app.json" in project root
};

export function gluxe(options: GluxeOptions = {}): Plugin {
  // Runtime-relevant app.json fields transcribed into the manifest by generateBundle.
  const runtimeConfig: { window?: unknown } = {};
  // Absolute path to the .ico declared in app.json. Emitted as a hashed dist asset and
  // recorded in the manifest for X11 window icon at startup. The Windows exe icon is
  // embedded separately by gluxe-build, which reads app.json from the source tree directly.
  let iconPath: string | undefined;
  return {
    name: "gluxe",
    async config(user, { mode }) {
      const root = user.root ?? process.cwd();
      const configPath = options.configPath ?? "app.json";
      const app = JSON.parse(await fs.promises.readFile(path.resolve(root, configPath), "utf8"));
      if (!app?.bundle?.entry) {
        throw new Error(`Entry point not found in ${configPath}`);
      }
      if (app.window !== undefined) {
        runtimeConfig.window = app.window;
      }
      if (app.icon !== undefined) {
        if (typeof app.icon !== "string" || !app.icon.toLowerCase().endsWith(".ico")) {
          throw new Error(
            `app.json "icon" must be a path to a .ico file (got ${JSON.stringify(app.icon)})`,
          );
        }
        // Relative to the app.json directory. Note: gluxe-build always reads
        // CARGO_MANIFEST_DIR/app.json, so with a non-default configPath the
        // Windows exe icon comes from the crate root's app.json, not this one.
        iconPath = path.resolve(path.dirname(path.resolve(root, configPath)), app.icon);
        if (!fs.existsSync(iconPath)) {
          throw new Error(`app.json "icon" not found: ${iconPath}`);
        }
      }
      return {
        build: {
          outDir: app.bundle?.outDir ?? "dist",
          // Explicit user setting wins; dev mode stays unminified; other modes (e.g. staging) minify.
          minify: user.build?.minify ?? mode !== "development",
          assetsInlineLimit: 0, // always emit separate asset files (never data: URI)
          modulePreload: false,
          rolldownOptions: {
            input: app.bundle?.entry ?? "index.tsx",
            output: {
              format: "iife", // self-contained for Boa
            },
          },
        },
        // Prefix every emitted asset URL with "asset://" so Rust can identify
        // them as EmbeddedAssets references (stripped to bare path before load).
        experimental: { renderBuiltUrl: (f) => `asset://${f}` },
      };
    },
    buildStart() {
      // addWatchFile is only valid in build-phase hooks (not generateBundle).
      if (iconPath) {
        this.addWatchFile(iconPath);
      }
    },
    generateBundle(_, bundle) {
      const entryChunks = Object.values(bundle).filter(
        (file) => file.type === "chunk" && file.isEntry,
      );
      if (entryChunks.length !== 1) {
        throw new Error(`gluxe expected exactly one entry chunk, found ${entryChunks.length}`);
      }
      let iconFileName: string | undefined;
      if (iconPath) {
        const ref = this.emitFile({
          type: "asset",
          name: path.basename(iconPath), // `name` (not `fileName`) → hashed assets/[name]-[hash].ico
          source: new Uint8Array(fs.readFileSync(iconPath)), // re-read each build so watch picks up edits
        });
        iconFileName = this.getFileName(ref);
      }
      this.emitFile({
        type: "asset",
        fileName: BUNDLE_MANIFEST_FILE,
        source: `${JSON.stringify(
          {
            // Spread first so hashed entry/icon paths can't be shadowed by a runtimeConfig key.
            ...runtimeConfig,
            version: 1,
            entry: entryChunks[0].fileName,
            ...(iconFileName ? { icon: iconFileName } : {}),
          },
          null,
          2,
        )}\n`,
      });
    },
  };
}
