import { defineConfig } from "tsdown";

export default defineConfig({
  entry: ["./src/index.ts"],
  outDir: "./dist",
  dts: true,
  sourcemap: true,
  deps: {
    neverBundle: ["gunshi"],
  },
});
