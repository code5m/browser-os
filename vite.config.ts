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
  build: {
    // M0-4.b：原先全部打进单个 503 kB 的入口 chunk（触发 Vite 500 kB 告警）。
    // 按依赖来源拆分：xterm 体积最大且只在终端视图用到，单独成 chunk 后
    // 入口 chunk 显著下降，且便于后续按视图懒加载。
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes("node_modules")) {
            return undefined;
          }
          if (id.includes("@xterm")) {
            return "xterm";
          }
          if (id.includes("/vue/") || id.includes("@vue") || id.includes("pinia")) {
            return "vue-vendor";
          }
          if (id.includes("@tauri-apps")) {
            return "tauri-vendor";
          }
          return "vendor";
        },
      },
    },
  },
});
