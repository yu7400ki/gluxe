import baseConfig from "@gluxe/config/oxlint/base.ts";
import { defineConfig } from "oxlint";

export default defineConfig({
  extends: [baseConfig],
  rules: {
    // Test utilities write files sequentially (mkdir before writeFile) — order matters.
    "no-await-in-loop": "off",
    // React components defined inside test cases for localised test setup;
    // hoisting them out would scatter context across the file.
    "unicorn/consistent-function-scoping": "off",
  },
});
