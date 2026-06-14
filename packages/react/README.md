# @gluxe/react

The JavaScript/React side of [gluxe](../../README.md): the `react-reconciler`
host config, the built-in primitives, the Vite plugin that bundles your app, and
the bridge for calling into the Rust runtime.

This package is what you `import` from in app code. The native runtime is the
separate `gluxe` Rust crate; the two are designed to work together.

## Install

```sh
npm install @gluxe/react react
```

## Entry points

| Import                | Contents                                                         |
| --------------------- | ---------------------------------------------------------------- |
| `@gluxe/react`        | Primitives, `registerRootComponent`, `invoke`, native components |
| `@gluxe/react/window` | Runtime window API (`setWindowTitle`)                            |
| `@gluxe/react/vite`   | The Vite plugin that bundles the app for the runtime             |

## Rendering a root

```tsx
import { registerRootComponent, View, Text } from "@gluxe/react";

function App() {
  return (
    <View style={{ flex: 1, padding: 16 }}>
      <Text>Hello</Text>
    </View>
  );
}

registerRootComponent(App);
```

## Primitives

`View`, `Text`, `Image`, and `TextInput` are exported as host element types and
augment React's JSX `IntrinsicElements`, so they are also usable as lowercase-free
JSX tags with full type checking.

```tsx
<View style={{ flexDirection: "row", gap: 8 }} onClick={(e) => console.log(e.x, e.y)}>
  <Text style={{ fontSize: 16, color: "#1d3557" }}>Label</Text>
  <Image src={logo} style={{ width: 24, height: 24 }} />
  <TextInput value={text} onChangeText={setText} onSubmit={save} />
</View>
```

Styles are plain objects with a CSS-like shape: flexbox/grid layout,
`px`/`rem`/`%`/`auto` units, hex/rgb/hsl/named colors, borders, box shadows, text
styling, `_hover`/`_active` states, and `transition`. See the exported
`StyleProps` type for the full set of fields.

## Calling the runtime

`invoke` calls a plugin command registered on the Rust side and resolves with its
result:

```ts
import { invoke } from "@gluxe/react";

const result = await invoke<string>("plugin|command", { arg: 1 });
```

`invokeStream` is the streaming counterpart — it returns a `GluxeStream` you can
consume with `for await` or `getReader()`:

```ts
import { invokeStream } from "@gluxe/react";

for await (const chunk of invokeStream<string>("plugin|stream", { path })) {
  console.log(chunk);
}
```

The window API lives under the `@gluxe/react/window` subpath:

```ts
import { setWindowTitle } from "@gluxe/react/window";
await setWindowTitle("New title");
```

## Native components

`nativeComponent` wraps a GPUI component registered in Rust (via
`Component::new("Name", ...)`) as a typed React component:

```tsx
import { nativeComponent } from "@gluxe/react";

const Badge = nativeComponent<{ count: number }>("Badge");

<Badge count={3} style={{ padding: 4 }} />;
```

## Vite plugin

`@gluxe/react/vite` reads `app.json`, bundles your entry into a single
self-contained chunk the runtime can evaluate, and writes a manifest (entry path,
window config, icon) into the output directory:

```ts
import react from "@vitejs/plugin-react";
import { gluxe } from "@gluxe/react/vite";
import { defineConfig } from "vite";

export default defineConfig({ plugins: [react(), gluxe()] });
```
