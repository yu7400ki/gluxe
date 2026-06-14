# @gluxe/cli

The command-line tool for [gluxe](../../README.md) apps. It wraps the two halves
of a build — bundling the JS with Vite and compiling the native binary with
Cargo — behind a few commands.

The binary is named `gluxe`.

## Commands

```sh
gluxe build      # bundle the JS, then compile the binary
gluxe run        # build, then launch the app
gluxe start      # development: watch + rebuild + hot reload
```

| Command | Description                                                                                                         |
| ------- | ------------------------------------------------------------------------------------------------------------------- |
| `build` | Runs the app's bundle build, then `cargo build`.                                                                    |
| `run`   | Same as `build`, then launches the compiled app.                                                                    |
| `start` | Runs the watch build, waits for the first bundle, then launches a debug build that reloads when the bundle changes. |

## Options

`build` and `run`:

| Option             | Default | Description                                |
| ------------------ | ------- | ------------------------------------------ |
| `--project <path>` | `.`     | Root of the project (where `app.json` is). |
| `--release`        | off     | Build the binary in release mode.          |

`start`:

| Option             | Default | Description          |
| ------------------ | ------- | -------------------- |
| `--project <path>` | `.`     | Root of the project. |

## How it reads the project

The commands are driven by `app.json` in the project root — they run the build
commands declared there (`bundle.build` for `build`/`run`, `dev.build` for
`start`) rather than assuming a particular bundler invocation. The only contract
for the dev build is that it keeps running and rewrites the output directory on
every change.

When `start` is running, the JS watcher and the native process are tied together:
quitting one (or pressing Ctrl+C) stops the other.
