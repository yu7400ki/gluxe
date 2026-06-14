// gluxe fs module — typed wrappers around the "fs" plugin commands.
//
// Usage:
//   import { fs } from "@gluxe/plugin-fs";
//
//   const text    = await fs.readTextFile("path/to/file.txt");
//   await fs.writeTextFile("out.txt", "hello");
//   const entries = await fs.readDir(".");          // DirEntry[]
//   const ok      = await fs.exists("path");        // boolean
//   await fs.mkdir("new/dir");                       // creates parents too
//   await fs.remove("path");                         // file or empty directory
//   const meta    = await fs.metadata("path");       // Metadata
//   const cwd     = await fs.cwd();                  // absolute working dir
//   const home    = await fs.homeDir();              // home directory
//   const abs     = await fs.canonicalize("./foo");  // absolute normalised path

import { type GluxeStream, invoke, invokeStream } from "gluxe";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** An entry returned by `readDir`. */
export interface DirEntry {
  /** File or directory name (without the directory path). */
  name: string;
  /** True when the entry is a directory. */
  isDir: boolean;
  /** True when the entry is a regular file. */
  isFile: boolean;
  /** Size in bytes. */
  size: number;
  /** Last-modified time as milliseconds since the Unix epoch, or null when
   *  the platform does not expose this field. */
  modified: number | null;
}

/** Detailed metadata for a single path, returned by `metadata`. */
export interface Metadata {
  /** True when the path refers to a directory. */
  isDir: boolean;
  /** True when the path refers to a regular file. */
  isFile: boolean;
  /** True when the path itself is a symbolic link (not its target). */
  isSymlink: boolean;
  /** Size in bytes. */
  size: number;
  /** True when the file permissions mark it as read-only. */
  readonly: boolean;
  /** Last-modified time as ms since the Unix epoch, or null. */
  modified: number | null;
  /** Last-accessed time as ms since the Unix epoch, or null. */
  accessed: number | null;
  /** Creation time as ms since the Unix epoch, or null. */
  created: number | null;
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/** Read the entire contents of a text file. */
export async function readTextFile(path: string): Promise<string> {
  return invoke<string>("fs|readTextFile", { path });
}

/**
 * Stream a text file line by line — each chunk is one line (without its
 * trailing newline). The file is read lazily, so large files are never fully
 * buffered. Breaking out of the `for await` loop (or calling `cancel()` on a
 * reader) stops the native read.
 *
 * @example
 * for await (const line of fs.readFileStream("big.log")) {
 *   console.log(line);
 * }
 */
export function readFileStream(path: string): GluxeStream<string> {
  return invokeStream<string>("fs|readFileStream", { path });
}

/**
 * Write `contents` to a file, creating it if it does not exist or truncating
 * it if it does.
 */
export async function writeTextFile(path: string, contents: string): Promise<void> {
  await invoke<null>("fs|writeTextFile", { path, contents });
}

/** List the entries of a directory. */
export async function readDir(path: string): Promise<DirEntry[]> {
  return invoke<DirEntry[]>("fs|readDir", { path });
}

/** Return `true` when the path exists (file or directory). */
export async function exists(path: string): Promise<boolean> {
  return invoke<boolean>("fs|exists", { path });
}

/**
 * Create a directory at `path`, including any necessary parent directories
 * (equivalent to `mkdir -p`).
 */
export async function mkdir(path: string): Promise<void> {
  await invoke<null>("fs|mkdir", { path });
}

/**
 * Remove a file or an empty directory at `path`.
 * Throws if the path does not exist or the directory is not empty.
 */
export async function remove(path: string): Promise<void> {
  await invoke<null>("fs|remove", { path });
}

/** Return detailed metadata for the file or directory at `path`. */
export async function metadata(path: string): Promise<Metadata> {
  return invoke<Metadata>("fs|metadata", { path });
}

/** Return the process's current working directory as an absolute path. */
export async function cwd(): Promise<string> {
  return invoke<string>("fs|cwd", {});
}

/** Return the current user's home directory as an absolute path. */
export async function homeDir(): Promise<string> {
  return invoke<string>("fs|homeDir", {});
}

/**
 * Resolve `path` to an absolute, normalised path with symlinks resolved and
 * `..` / `.` segments removed.  Throws when the path does not exist.
 */
export async function canonicalize(path: string): Promise<string> {
  return invoke<string>("fs|canonicalize", { path });
}
