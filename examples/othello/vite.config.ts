import { gluxeRouter } from "@gluxe/router/vite";
import react from "@vitejs/plugin-react";
import { gluxe } from "gluxe/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react(), gluxe(), gluxeRouter()],
});
