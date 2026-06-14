import fs from "node:fs";
import path from "node:path";

import type { Plugin, ViteDevServer } from "vite";

const VIRTUAL_ID = "virtual:@gluxe/router/routes";
const RESOLVED_ID = `\0${VIRTUAL_ID}`;

const ROUTE_FILE_RE = /\.(tsx|ts|jsx|js)$/;
const PARAM_RE = /^\[(.+)\]$/;

export type GluxeRouterOptions = {
  /** Directory scanned for route files, relative to the Vite root (default: "src/routes"). */
  routesDir?: string;
};

/** Intermediate tree built from the filesystem, before code generation. */
interface RouteNode {
  id: string;
  path?: string;
  index?: boolean;
  /** Absolute path of the module whose default export is the component. */
  file?: string;
  children?: RouteNode[];
}

/** `[id]` → `:id`; anything else is a literal segment. */
function toSegment(name: string): string {
  const param = PARAM_RE.exec(name);
  return param ? `:${param[1]}` : name;
}

interface ScannedDir {
  layoutFile?: string;
  nodes: RouteNode[];
  files: string[]; // every route file found, for watching
}

function scanDir(dirAbs: string, routePrefix: string): ScannedDir {
  const result: ScannedDir = { nodes: [], files: [] };
  const entries = fs
    .readdirSync(dirAbs, { withFileTypes: true })
    .toSorted((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));

  const fileBasenames = new Map<string, string>(); // basename → absolute path
  const dirNames: string[] = [];

  for (const entry of entries) {
    if (entry.name.startsWith(".")) continue;
    if (entry.isDirectory()) {
      if (entry.name.startsWith("_")) continue;
      dirNames.push(entry.name);
      continue;
    }
    if (!ROUTE_FILE_RE.test(entry.name)) continue;
    const basename = entry.name.replace(ROUTE_FILE_RE, "");
    if (basename.startsWith("_") && basename !== "_layout") continue;
    const existing = fileBasenames.get(basename);
    if (existing) {
      throw new Error(
        `[gluxe-router] Conflicting route files "${path.basename(existing)}" and ` +
          `"${entry.name}" in ${dirAbs} map to the same route.`,
      );
    }
    fileBasenames.set(basename, path.join(dirAbs, entry.name));
  }

  for (const [basename, fileAbs] of fileBasenames) {
    result.files.push(fileAbs);
    if (basename === "_layout") {
      result.layoutFile = fileAbs;
    } else if (basename === "index") {
      result.nodes.push({ id: `${routePrefix}/`, index: true, file: fileAbs });
    } else if (basename === "404") {
      result.nodes.push({ id: `${routePrefix}/*`, path: "*", file: fileAbs });
    } else {
      if (dirNames.includes(basename)) {
        throw new Error(
          `[gluxe-router] "${basename}${path.extname(fileAbs)}" and directory ` +
            `"${basename}/" coexist in ${dirAbs}. Move the file to "${basename}/index" ` +
            `or "${basename}/_layout" instead.`,
        );
      }
      const segment = toSegment(basename);
      result.nodes.push({ id: `${routePrefix}/${segment}`, path: segment, file: fileAbs });
    }
  }

  for (const dirName of dirNames) {
    const segment = toSegment(dirName);
    const child = scanDir(path.join(dirAbs, dirName), `${routePrefix}/${segment}`);
    result.files.push(...child.files);
    // Skip dirs with no page routes even if they have a _layout:
    // a layout never matches on its own (see matcher.ts flattenRoutes).
    if (child.nodes.length === 0) continue;
    result.nodes.push({
      id: `${routePrefix}/${segment}`,
      path: segment,
      file: child.layoutFile, // pass-through node when the dir has no _layout
      children: child.nodes,
    });
  }

  return result;
}

function buildRouteTree(routesDir: string): { nodes: RouteNode[]; files: string[] } {
  const root = scanDir(routesDir, "");
  if (root.layoutFile) {
    return {
      nodes: [{ id: "/_layout", file: root.layoutFile, children: root.nodes }],
      files: root.files,
    };
  }
  return { nodes: root.nodes, files: root.files };
}

function generateRoutesModule(nodes: RouteNode[]): string {
  const imports: string[] = [];
  const importVarByFile = new Map<string, string>();

  const importVar = (file: string): string => {
    let name = importVarByFile.get(file);
    if (!name) {
      name = `R${importVarByFile.size}`;
      importVarByFile.set(file, name);
      // JSON.stringify + forward slashes keep Windows paths valid as specifiers.
      imports.push(`import ${name} from ${JSON.stringify(file.replace(/\\/g, "/"))};`);
    }
    return name;
  };

  const serialize = (node: RouteNode, indent: string): string => {
    const fields: string[] = [`id: ${JSON.stringify(node.id)}`];
    if (node.path !== undefined) fields.push(`path: ${JSON.stringify(node.path)}`);
    if (node.index) fields.push("index: true");
    if (node.file) fields.push(`component: ${importVar(node.file)}`);
    if (node.children && node.children.length > 0) {
      const children = node.children.map((c) => serialize(c, `${indent}  `)).join(`,\n${indent}  `);
      fields.push(`children: [\n${indent}  ${children},\n${indent}]`);
    }
    return `{ ${fields.join(", ")} }`;
  };

  const body = nodes.map((node) => serialize(node, "  ")).join(",\n  ");
  return `${imports.join("\n")}\n\nexport const routes = [\n  ${body},\n];\n`;
}

/**
 * File-based router Vite plugin. Scans `src/routes/` (Next.js pages-style:
 * `index.tsx`, `about.tsx`, `users/[id].tsx`, `_layout.tsx`, `404.tsx`) and
 * exposes the route tree as `virtual:@gluxe/router/routes` with eager static
 * imports so everything stays in the single IIFE chunk Boa requires.
 */
export function gluxeRouter(options: GluxeRouterOptions = {}): Plugin {
  let routesDir = "";
  let server: ViteDevServer | undefined;

  const invalidate = (): void => {
    const mod = server?.moduleGraph.getModuleById(RESOLVED_ID);
    if (mod && server) {
      server.moduleGraph.invalidateModule(mod);
      server.ws.send({ type: "full-reload" });
    }
  };

  return {
    name: "gluxe-router",
    configResolved(config) {
      routesDir = path.resolve(config.root, options.routesDir ?? "src/routes");
    },
    resolveId(id) {
      if (id === VIRTUAL_ID) return RESOLVED_ID;
    },
    configureServer(s) {
      server = s;
      const isInRoutesDir = (file: string): boolean => {
        // path.relative is separator-safe and rejects siblings like "src/routes-v2"
        // that a bare startsWith(routesDir) would accept.
        const rel = path.relative(routesDir, file);
        return !rel.startsWith("..") && !path.isAbsolute(rel);
      };
      const onChange = (file: string): void => {
        if (isInRoutesDir(file)) invalidate();
      };
      s.watcher.on("add", onChange);
      s.watcher.on("unlink", onChange);
    },
    load(id) {
      if (id !== RESOLVED_ID) return;
      if (!fs.existsSync(routesDir)) {
        throw new Error(
          `[gluxe-router] Routes directory not found: ${routesDir}. ` +
            `Create it (or set the "routesDir" plugin option).`,
        );
      }
      this.addWatchFile(routesDir);
      const { nodes, files } = buildRouteTree(routesDir);
      for (const file of files) this.addWatchFile(file);
      return generateRoutesModule(nodes);
    },
  };
}
