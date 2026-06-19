// Shared dark-mode palette for the showcase. @gluxe/ui ships zero styles, so
// every visual here is supplied by the example — this is the single source of
// colour truth that the section components reference.

export const theme = {
  bg: "#0f1115",
  surface: "#181b22",
  surfaceHigh: "#20242d",
  border: "#2a2f3a",
  borderHigh: "#3a4150",
  text: "#e6e9ef",
  textMuted: "#9aa3b2",
  accent: "#6ea8fe",
  accentBright: "#8fbcff",
  accentDim: "#3d6fd6",
  accentText: "#0a1020",
  track: "#2a2f3a",
  danger: "#ff6b6b",
} as const;

// A keyboard focus ring shared across the showcase. @gluxe/ui parts are the
// focusable nodes, so this is passed to the part component itself (not an inner
// render-prop View). `_focusVisible` shows it only during keyboard use (Tab /
// arrow keys), staying out of the way for mouse clicks. `borderRadius` is set
// per-component to match the part's shape so the ring hugs it cleanly.
export function focusRing(borderRadius: number): {
  _focusVisible: { borderWidth: number; borderColor: string; borderRadius: number };
} {
  return {
    _focusVisible: { borderWidth: 2, borderColor: theme.accent, borderRadius },
  };
}
