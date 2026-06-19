// The value exposed on a host element `ref` (`ref.current`).

/**
 * The value exposed on a host element `ref` (`ref.current`). Carries the
 * Rust-side element id plus imperative focus controls.
 *
 * @example
 * const ref = useRef<GpuiInstance>(null);
 * <View ref={ref} tabIndex={0} />;
 * ref.current?.focus();
 */
export interface GpuiInstance {
  /** Rust-side ElementId of this host element. */
  readonly id: number;
  /** Move keyboard focus to this element (no-op unless it is focusable). */
  focus(): Promise<void>;
  /** Remove keyboard focus from this element (only if it currently holds it). */
  blur(): Promise<void>;
}
