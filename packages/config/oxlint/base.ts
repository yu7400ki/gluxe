import { defineConfig } from "oxlint";

export default defineConfig({
  plugins: ["typescript", "unicorn", "oxc"],
  categories: {
    correctness: "error",
    perf: "warn",
    suspicious: "error",
  },
  rules: {
    "no-console": "off",
    // `_hover` / `_active` / `_focus` / `_focusVisible` are gluxe style-prop
    // pseudo-selector keys (public API); the leading underscore is intentional,
    // so exempt them from no-underscore-dangle for consumers that read them.
    "no-underscore-dangle": ["error", { allow: ["_hover", "_active", "_focus", "_focusVisible"] }],
  },
  env: {
    builtin: true,
  },
});
