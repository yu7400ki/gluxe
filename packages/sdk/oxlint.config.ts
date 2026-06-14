import baseConfig from "@gluxe/config/oxlint/base.ts";
import { defineConfig } from "oxlint";

export default defineConfig({
  extends: [baseConfig],
  rules: {
    // Bridge globals (__bridge, __invoke, __dispatchEvent, etc.) are part of the
    // Rust↔JS contract and cannot be renamed.
    "no-underscore-dangle": "off",
  },
});
