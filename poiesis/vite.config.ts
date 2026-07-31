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
      ignored: [
        "**/src-tauri/**",
        "**/.claude/worktrees/**", // 隔离 worktree，防止 HMR 误触发
      ],
    },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          "vendor-core": ["react", "react-dom", "zustand", "sonner"],
          "vendor-flow": ["@xyflow/react"],
          "vendor-dnd": ["@dnd-kit/core", "@dnd-kit/utilities"],
          "vendor-layout": ["dagre"],
          "vendor-monaco": ["monaco-editor", "@monaco-editor/react"],
        },
      },
    },
    chunkSizeWarningLimit: 200,
  },
}));
