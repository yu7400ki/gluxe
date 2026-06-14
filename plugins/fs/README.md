# gluxe filesystem plugin

A filesystem plugin for [gluxe](../../README.md): a set of native commands for
reading and writing files from JavaScript.

It has two halves — the Rust plugin (`gluxe-plugin-fs`) that registers the
commands, and the JS package (`@gluxe/plugin-fs`) with typed wrappers around them.

## Setup

Register the plugin on the Rust side:

```rust
fn main() {
    gluxe::RuntimeBuilder::new()
        .plugin(gluxe_plugin_fs::plugin())
        .run(gluxe::embedded_dist!());
}
```

Then call it from JS:

```ts
import { fs } from "@gluxe/plugin-fs";

const text = await fs.readTextFile("notes.txt");
await fs.writeTextFile("out.txt", "hello");
const entries = await fs.readDir(".");
```

## API

| Function                        | Returns               | Description                                            |
| ------------------------------- | --------------------- | ------------------------------------------------------ |
| `readTextFile(path)`            | `string`              | Read a text file in full.                              |
| `readFileStream(path)`          | `GluxeStream<string>` | Stream a file line by line (lazy; not fully buffered). |
| `writeTextFile(path, contents)` | `void`                | Write a file, creating or truncating it.               |
| `readDir(path)`                 | `DirEntry[]`          | List directory entries.                                |
| `exists(path)`                  | `boolean`             | Whether the path exists.                               |
| `mkdir(path)`                   | `void`                | Create a directory, including parents.                 |
| `remove(path)`                  | `void`                | Remove a file or empty directory.                      |
| `metadata(path)`                | `Metadata`            | Detailed metadata for a path.                          |
| `cwd()`                         | `string`              | Current working directory (absolute).                  |
| `homeDir()`                     | `string`              | The current user's home directory.                     |
| `canonicalize(path)`            | `string`              | Resolve to an absolute, normalized path.               |

`DirEntry` carries `name`, `isDir`, `isFile`, `size`, and `modified`; `Metadata`
adds `isSymlink`, `readonly`, `accessed`, and `created`. Times are milliseconds
since the Unix epoch, or `null` when the platform does not expose them.

Streaming a large log file:

```ts
for await (const line of fs.readFileStream("big.log")) {
  console.log(line);
}
```
