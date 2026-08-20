import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { host: "127.0.0.1", port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_"],
  // The Windows app is rendered by the system WebView2 runtime. Keep the
  // generated syntax below the baseline that the installer supports so an
  // older (but still supported) runtime cannot fail before React mounts.
  build: { target: "es2018", minify: "esbuild", sourcemap: true },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    // Without this Vitest short-circuits CSS imports to an empty string, which
    // silently turns every stylesheet assertion into a vacuous pass.
    css: true,
  },
});
