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
  },
  env: {
    builtin: true,
  },
});
