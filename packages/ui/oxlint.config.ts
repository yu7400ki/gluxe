import baseConfig from "@gluxe/config/oxlint/base.ts";
import { defineConfig } from "oxlint";

export default defineConfig({
  extends: [baseConfig],
  rules: {
    // React components are defined inside test cases for localised setup;
    // hoisting them out would scatter context across the file.
    "unicorn/consistent-function-scoping": "off",
  },
});
