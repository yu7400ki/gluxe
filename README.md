# gluxe

A desktop UI framework for writing apps in React that render to a native,
GPU-drawn window — without a browser or web view.

You write ordinary React (JSX + Hooks). A Rust runtime executes that JavaScript
with the [Boa](https://github.com/boa-dev/boa) engine and paints it with
[GPUI](https://www.gpui.rs/) (the GPU UI layer behind the Zed editor). There is
no DOM. The finished app is a single native binary with the JS bundle embedded
inside it, so there is no Node.js or separate runtime to install on the end
user's machine.

gluxe is early and experimental. The API will change.

```tsx
import { View, Text } from "gluxe";
import { useState } from "react";

export default function Counter() {
  const [count, setCount] = useState(0);
  return (
    <View style={{ flex: 1, alignItems: "center", gap: 12, padding: 16 }}>
      <Text style={{ fontSize: 24 }}>Count: {count}</Text>
      <View
        style={{
          padding: 8,
          borderRadius: 4,
          backgroundColor: "#98c1d9",
          _hover: { backgroundColor: "#5fa8d3" },
          _active: { backgroundColor: "#3d8ab8" },
        }}
        onClick={() => setCount((c) => c + 1)}
      >
        <Text>+1</Text>
      </View>
    </View>
  );
}
```

## What you get

- React 19 with function components and Hooks, rendered to native GPUI views via
  a `react-reconciler` host config.
- Built-in primitives: `<View>`, `<Text>`, `<Image>`, `<TextInput>`.
- A CSS-like style system: flexbox and a uniform-track CSS grid;
  `px`/`rem`/`%`/`auto` units; hex/rgb/hsl/named colors; borders, box shadows,
  text styling; `_hover`/`_active` states; and transitions.
- Mouse and keyboard events with DOM-style handlers (`onClick`, `onKeyDown`, …),
  `autoFocus`, and text input.
- A typed plugin system for calling Rust from JS, plus a way to register your own
  native GPUI components as React elements.
- Hot reload during development (`gluxe start`).
- Optional custom titlebar: hide the system one and define drag/control regions
  in JSX.
- TypeScript types for everything, including JSX `IntrinsicElements`.

## How it works

```
  Your React app (JSX + Hooks)
          │  react-reconciler host config
          ▼
  JS bundle  ──bundled by Vite──►  embedded into the native binary
          │
          ▼  executed by the Boa JavaScript engine (Rust)
  gluxe runtime  ──drives──►  GPUI view tree  ──►  GPU-rendered window
```

The whole stack is Rust: the JavaScript engine
([Boa](https://github.com/boa-dev/boa)), the GPU UI layer
([GPUI](https://www.gpui.rs/), the framework behind the Zed editor), and the
runtime that bridges them. At build time, Vite bundles your React code and a Rust
build step embeds the bundle into the executable; at run time, the runtime
evaluates that JavaScript and a React reconciler host config translates React's
tree operations into native view updates every frame.

## Getting started

### Prerequisites

- **Rust** (stable) with Cargo — compiles the native runtime and produces the binary.
- **Node.js + [pnpm](https://pnpm.io/)** — used at _build time_ to bundle your
  React code with Vite. (Not required to run the finished app.)
- Platform GPU/graphics toolchain as required by GPUI for your OS.

### Project shape

A gluxe app is both a Node package (the React UI) and a Cargo binary (the native
host). A minimal app looks like this:

```
my-app/
├── app.json          # window + bundle configuration
├── package.json      # depends on "gluxe" and "react"
├── vite.config.ts    # uses the gluxe Vite plugin
├── index.tsx         # registers the root component
├── App.tsx           # your React UI
├── Cargo.toml        # depends on the gluxe crate
└── src/main.rs        # native entry point
```

**`index.tsx`** — register your root component:

```tsx
import { registerRootComponent } from "gluxe";
import App from "./App";

registerRootComponent(App);
```

**`vite.config.ts`** — add the React and gluxe plugins:

```ts
import react from "@vitejs/plugin-react";
import { gluxe } from "gluxe/vite";
import { defineConfig } from "vite";

export default defineConfig({ plugins: [react(), gluxe()] });
```

**`src/main.rs`** — embed the built bundle and run:

```rust
fn main() {
    gluxe::RuntimeBuilder::new()
        .run(gluxe::embedded_dist!());
}
```

**`app.json`** — window settings and how to build the JS bundle:

```json
{
  "window": { "width": 800, "height": 600, "title": "My App" },
  "bundle": {
    "entry": "index.tsx",
    "outDir": "dist",
    "build": { "command": "pnpm", "args": ["build"] }
  },
  "dev": {
    "build": { "command": "pnpm", "args": ["dev"] }
  }
}
```

### Run it

Using the CLI (`@gluxe/cli`, the `gluxe` command):

```sh
pnpm install      # install JS dependencies

gluxe start       # development: watch + rebuild + hot reload
gluxe run         # build the JS bundle and launch the app
gluxe build       # build the JS bundle and compile the release binary
```

Under the hood these wrap `vite build` and `cargo build`/`cargo run`, so you can
also drive them directly:

```sh
pnpm build        # bundle JS → dist/
cargo run         # compile the native binary and launch the window
```

The browseable [`examples/`](examples/) directory contains ready-to-run apps —
`counter`, `file-explorer`, and `othello`.

## Building UIs

### Primitives

Import host elements from `gluxe` and use them as JSX:

| Element       | Description                              |
| ------------- | ---------------------------------------- |
| `<View>`      | Layout container (flexbox / grid)        |
| `<Text>`      | Text content                             |
| `<Image>`     | Bundled asset, local file, or remote URL |
| `<TextInput>` | Editable single-line text field          |

### Styling

Styles are plain objects passed via the `style` prop:

```tsx
<View
  style={{
    display: "flex",
    flexDirection: "column",
    gap: 12,
    padding: 16,
    borderRadius: 8,
    backgroundColor: "#3d5a80",
    boxShadow: "lg",
    transition: { property: "all", duration: 200, easing: "ease" },
    _hover: { backgroundColor: "#5fa8d3" },
  }}
/>
```

- **Units:** bare numbers and `"px"` are pixels; `"rem"`, `"%"`, and `"auto"`
  are also supported where applicable.
- **Colors:** hex (3/4/6/8-digit), `rgb()`/`rgba()`, `hsl()`/`hsla()`, all CSS
  named colors, and `"transparent"`.
- **Layout:** flexbox plus a uniform-track CSS grid.
- **Pseudo-states:** `_hover` and `_active`.
- **Transitions:** animate length/color/scalar props over time.

### Events

DOM-style handlers receive a typed event object:

```tsx
<View onClick={(e) => console.log(e.x, e.y)} />
<View autoFocus onKeyDown={(e) => { if (e.key === "enter") submit(); }} />
<TextInput value={text} onChangeText={setText} onSubmit={save} />
```

## Native integration

### Calling Rust from JavaScript

Plugins expose typed commands to JS. Define them in Rust with the `#[command]`
macro:

```rust
use gluxe::command;

#[command]
fn greet(name: String) -> Result<String, String> {
    Ok(format!("Hello, {name}!"))
}

fn main() {
    let plugin = gluxe::PluginBuilder::new("demo")
        .commands(gluxe::commands![greet])
        .build();

    gluxe::RuntimeBuilder::new()
        .plugin(plugin)
        .run(gluxe::embedded_dist!());
}
```

Call them from JS with `invoke` (and `invokeStream` for streaming commands):

```ts
import { invoke } from "gluxe";

const message = await invoke<string>("demo|greet", { name: "world" });
```

Commands come in three flavors — synchronous, `async` (background thread), and
`stream` (push values to a JS `ReadableStream`).

### Native components

Register your own GPUI components as React host elements with
`Component::new(...)` on the Rust side and `nativeComponent(...)` on the JS side,
then use them in JSX like any built-in primitive.

### Filesystem plugin

The first-party filesystem plugin (`@gluxe/plugin-fs` / `gluxe-plugin-fs`)
provides a `fs` API to read directories and files:

```ts
import { fs } from "@gluxe/plugin-fs";

const entries = await fs.readDir("/some/path");
const text = await fs.readTextFile("/some/file.txt");
```

## Configuration (`app.json`)

| Field                 | Purpose                                                      |
| --------------------- | ------------------------------------------------------------ |
| `window.width/height` | Initial window size (default 800×600)                        |
| `window.title`        | Initial titlebar text                                        |
| `window.titlebar`     | Set `false` to hide the system titlebar (build a custom one) |
| `icon`                | Application icon — a `.ico` file (Windows & Linux/X11)       |
| `bundle.entry`        | JS entry file                                                |
| `bundle.outDir`       | Bundle output directory (embedded into the binary)           |
| `bundle.build`        | Command the CLI runs to produce the bundle                   |
| `dev.build`           | Watch-mode build command used by `gluxe start`               |

The window settings travel with the bundle, so changing them requires rebuilding
the JS (`pnpm build` / `gluxe build`). The window title can also be changed at
runtime via `setWindowTitle` from the `gluxe/window` export.

For a custom titlebar, set `window.titlebar: false` and mark drag/control regions
in JSX with the `windowControlArea` prop (`"drag"`, `"close"`, `"max"`, `"min"`).

## Repository layout

This is a pnpm + Cargo monorepo.

| Path                   | Package                                | Description                                        |
| ---------------------- | -------------------------------------- | -------------------------------------------------- |
| `crates/core`          | `gluxe`                                | The native runtime: JS engine + GPUI bridge (Rust) |
| `crates/macros`        | `gluxe-macros`                         | The `#[command]` proc-macro                        |
| `crates/build-support` | `gluxe-build`                          | `build.rs` helper for app projects                 |
| `plugins/fs`           | `gluxe-plugin-fs` / `@gluxe/plugin-fs` | Filesystem plugin                                  |
| `packages/sdk`         | `gluxe`                                | JS SDK: React host config, primitives, Vite plugin |
| `packages/router`      | `@gluxe/router`                        | File-based and code-based router                   |
| `packages/cli`         | `@gluxe/cli`                           | The `gluxe` command-line tool                      |
| `examples/`            | —                                      | `counter`, `file-explorer`, `othello`              |

### Optional Cargo features (core crate)

- **`http`** — enables a real HTTP client for loading remote `<Image>` URLs
  (`https://…`). Without it, remote images silently fail to load.
- **`intl`** _(default)_ — bundles ICU data for JavaScript `Intl` and
  locale-aware methods. Disable to shrink the binary if you don't need them.

## Platform support

Targets **Windows**, **macOS**, and **Linux** (X11/Wayland). Some
platform-specific notes:

- Custom (frameless) titlebars behave slightly differently per platform; on
  Linux without a compositor, frameless windows fall back to server decorations.
- The application icon is supported on Windows and Linux/X11.

## Developing gluxe itself

```sh
cargo build                      # compile the Rust workspace
cargo check                      # fast type-check
cargo test -p gluxe              # Rust unit tests
pnpm build                       # build the JS packages
```
