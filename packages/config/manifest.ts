// Contract for the bundle manifest (gluxe.manifest.json): written into the dist
// dir by @gluxe/react's Vite plugin and read back by the runtime (Rust) and the
// CLI's dev mode. Shared between the writer and the JS reader so the filename and
// shape can never drift.

/** Filename of the bundle manifest within the dist directory. */
export const BUNDLE_MANIFEST_FILE = "gluxe.manifest.json";

/** Shape of `gluxe.manifest.json`. */
export interface BundleManifest {
  version: number;
  /** Hashed dist-relative path of the JS entry chunk. */
  entry: string;
  /** Hashed dist-relative path of the app icon, when app.json declares one. */
  icon?: string;
  /** Window settings transcribed verbatim from app.json. */
  window?: unknown;
}
