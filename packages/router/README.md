# @gluxe/router

A small router for [gluxe](../../README.md) apps. Navigation runs against an
in-memory history (there is no URL bar), and routes can be declared either in code
or generated from the filesystem.

## Entry points

| Import                 | Contents                                          |
| ---------------------- | ------------------------------------------------- |
| `@gluxe/router`        | `Router`, `Outlet`, `Link`, hooks, route matching |
| `@gluxe/router/vite`   | The file-based routing Vite plugin                |
| `@gluxe/router/client` | Types for the generated virtual routes module     |

## Code-based routing

Pass a route tree to `<Router>`. A route with `children` is a layout that renders
its matched child through `<Outlet />`:

```tsx
import { Router, Outlet, Link } from "@gluxe/router";

const routes = [
  {
    path: "",
    component: Layout,
    children: [
      { index: true, component: Home },
      { path: "users/:id", component: User },
      { path: "*", component: NotFound },
    ],
  },
];

function Layout() {
  return (
    <View>
      <Link to="/">Home</Link>
      <Outlet />
    </View>
  );
}

function App() {
  return <Router routes={routes} />;
}
```

Path segments may be static (`"about"`), dynamic (`":id"`), or `"*"` (wildcard).

## File-based routing

The Vite plugin scans a routes directory (default `src/routes`) and generates the
route tree for you, then exposes it as a virtual module:

```ts
import { gluxeRouter } from "@gluxe/router/vite";
import { defineConfig } from "vite";

export default defineConfig({ plugins: [gluxeRouter()] });
```

```tsx
import { Router } from "@gluxe/router";
import { routes } from "virtual:@gluxe/router/routes";

function App() {
  return <Router routes={routes} />;
}
```

File-name conventions (Next.js pages-style):

| File                 | Route                         |
| -------------------- | ----------------------------- |
| `index.tsx`          | the parent path (index route) |
| `about.tsx`          | `/about`                      |
| `users/[id].tsx`     | `/users/:id`                  |
| `_layout.tsx`        | layout for its directory      |
| `404.tsx`            | wildcard / not-found route    |
| `_name.tsx`, `_dir/` | ignored (private)             |

Adding, removing, or renaming a route file triggers a reload in dev.

## Hooks

| Hook            | Returns                                                                        |
| --------------- | ------------------------------------------------------------------------------ |
| `useNavigate()` | `navigate(to, options?)` — go to a path, or pass a delta number (`-1` = back). |
| `useLocation()` | The current `Location` (`pathname`, `key`, `state`).                           |
| `useParams()`   | Dynamic path params captured during matching.                                  |
| `useHistory()`  | Lower-level history controls.                                                  |

```tsx
import { useNavigate, useParams, useLocation } from "@gluxe/router";

const navigate = useNavigate();
const { id } = useParams();
const { state } = useLocation();

navigate("/users/42", { state: { from: "list" } });
navigate(-1); // back
```
