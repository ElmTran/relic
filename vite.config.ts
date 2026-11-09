import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// 每个窗口一个入口，只加载自己需要的代码
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "es2022",
    rollupOptions: {
      input: {
        translate: "src/windows/translate/index.html",
        indicator: "src/windows/indicator/index.html",
        settings: "src/windows/settings/index.html",
      },
    },
  },
});
