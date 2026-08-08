import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://v2.tauri.app/start/frontend/vite/
const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    rollupOptions: {
      output: {
        // vite 8 内置 rolldown：manualChunks 仅接受函数形式（对象形式报
        // "Expected Function but received Object"）。函数形式按模块 id
        // 归属 vendor 分块，语义与旧对象形式等价。
        manualChunks(id: string): string | undefined {
          if (
            id.includes("/node_modules/react/") ||
            id.includes("/node_modules/react-dom/") ||
            id.includes("/node_modules/zustand/") ||
            id.includes("/node_modules/sonner/")
          ) {
            return "vendor-core";
          }
          if (id.includes("/node_modules/@xyflow/react/")) return "vendor-flow";
          if (id.includes("/node_modules/@dnd-kit/")) return "vendor-dnd";
          if (id.includes("/node_modules/dagre/")) return "vendor-layout";
          if (
            id.includes("/node_modules/monaco-editor/") ||
            id.includes("/node_modules/@monaco-editor/")
          ) {
            return "vendor-monaco";
          }
          return undefined;
        },
      },
    },
    chunkSizeWarningLimit: 200,
  },
}));
