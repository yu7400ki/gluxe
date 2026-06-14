import baseConfig from "@gluxe/config/oxlint/base.ts";
import { defineConfig } from "oxlint";

export default defineConfig({
  extends: [baseConfig],
  rules: {
    // Polling loop (waitForFirstBuild) is inherently sequential; parallelism is
    // meaningless when you're waiting for a single external condition.
    "no-await-in-loop": "off",
  },
});
