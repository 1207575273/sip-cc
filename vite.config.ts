import { defineConfig } from "vite";
import { readFileSync } from "fs";

const tauriConf = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf-8"));

export default defineConfig({
  root: "ui",
  define: {
    __APP_VERSION__: JSON.stringify(tauriConf.version),
  },
  build: {
    outDir: "../dist",
    emptyOutDir: true,
  },
  server: {
    port: 1420,
    strictPort: true,
  },
});
