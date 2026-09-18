import { defineConfig } from "vite";

// Tauri expects a fixed dev port and no screen clearing (keeps Rust errors visible).
export default defineConfig({
  root: "ui",
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    outDir: "../dist",
    emptyOutDir: true,
    target: "es2022",
  },
});
