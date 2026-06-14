import { gluxe } from "@gluxe/react/vite";
import { gluxeRouter } from "@gluxe/router/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react(), gluxe(), gluxeRouter()],
});
