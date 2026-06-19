// Public entry for @gluxe/react.
//
// Export-placement rule (mirrored by package.json "exports"):
// - The root entry holds the foundational, cross-cutting runtime surface every app
//   reaches for: the element primitives (View/Text/Image/TextInput), the
//   portal/root runtime, native-component declaration, and the plugin-call
//   primitives `invoke` / `invokeStream`. invoke/stream are the base RPC layer the
//   domain subpaths are themselves built on (window.ts and focus.ts dispatch
//   through `invoke`), so they belong at the root, not behind a subpath.
// - Subpaths carry what must stay out of the root: build-time-only code that can't
//   load in the runtime (`./vite`, which imports `vite`), and optional, narrowly
//   scoped domain APIs (`./window`, `./focus`, `./components`). These stay on
//   subpaths rather than being re-exported from the root so the root surface stays
//   small and stable: an app imports only the domains it uses, and adding a new
//   domain never grows what every importer pulls in. A new domain API gets its own
//   subpath; the root stays the foundational primitives.
//
// Relocating invoke/stream to a subpath was considered and rejected: they are the
// cross-cutting primitive the domain APIs depend on, and the export paths are a
// public contract.

export { View, Text, Image, TextInput } from "./primitives";
export type {
  ViewProps,
  TextProps,
  ImageProps,
  TextInputProps,
  StyleProps,
  TransitionValue,
  TransitionEasing,
  HostType,
  GpuiInstance,
  GpuiEventBase,
  GpuiMouseEvent,
  GpuiKeyboardEvent,
  GpuiFocusEvent,
  EventProps,
  TextChangeHandler,
  FloatingProps,
  FloatingArea,
  FloatingSide,
  FloatingAlign,
} from "./primitives";
export { createPortal, registerRootComponent } from "./runtime";
export { nativeComponent } from "./native";
export type { NativeComponentProps } from "./native";
export { invoke } from "./invoke";
export { invokeStream, GluxeStream } from "./stream";
export type { GluxeStreamReader } from "./stream";
