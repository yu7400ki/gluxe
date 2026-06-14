// Typed JSX wrapper for native GPUI components registered in Rust via `Component::new("Name", ...)`.
// React routes any string element type through the reconciler, so `createElement("Badge", props)`
// reaches `bridge.createInstance("Badge", ...)`. This wraps it in a typed function component.

import { createElement, type ReactNode } from "react";

import type { EventProps, StyleProps } from "./primitives";

/** Base props every native component accepts. */
export type NativeComponentProps = EventProps & {
  style?: StyleProps;
  children?: ReactNode;
};

/**
 * Create a typed JSX wrapper for a native component registered in Rust under `name`.
 * `P` describes the custom props your render function reads from `ctx.props`.
 */
export function nativeComponent<P extends object = Record<string, never>>(
  name: string,
): (props: P & NativeComponentProps) => ReactNode {
  const Component = (props: P & NativeComponentProps): ReactNode =>
    createElement(name, props as Record<string, unknown>);
  Component.displayName = name;
  return Component;
}
