import { defineConfig } from "tsdown";

export default defineConfig({
  entry: ["./js/index.ts"],
  outDir: "./dist",
  dts: true,
  sourcemap: true,
});
