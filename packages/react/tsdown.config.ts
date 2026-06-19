import { defineConfig } from "tsdown";

export default defineConfig({
  entry: ["./js/index.ts", "./js/vite.ts", "./js/window.ts", "./js/focus.ts", "./js/components.ts"],
  outDir: "./dist",
  dts: true,
  sourcemap: true,
  external: ["vite"],
});
