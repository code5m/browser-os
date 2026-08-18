import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1421,
    // 必须严格端口：tauri.conf.json 的 devUrl 固定指向 1421，
    // 若端口被占悄悄换端口，webview 会加载到旧 dist 且无任何报错（本次已踩坑）
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
