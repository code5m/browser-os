import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  // All 71 in-repository Vue SFCs use <script setup>; omit unused Options API
  // support from the production runtime rather than raising bundle-size limits.
  define: { __VUE_OPTIONS_API__: false },
  clearScreen: false,
  server: {
    port: 1421,
    // 必须严格端口：tauri.conf.json 的 devUrl 固定指向 1421，
    // 若端口被占悄悄换端口，webview 会加载到旧 dist 且无任何报错（本次已踩坑）
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
    // H01 修复（FINAL_HUMAN_ACCEPTANCE_H01_BLOCKER_REPAIR）：
    // 根因 = WEBVIEW_CACHE。组件从 src/components/* 经 git mv 迁到 src/capabilities/*/ui/
    // 发生在同一 dev 生命周期内，WebKitGTK 磁盘缓存保留了迁移前的旧模块
    // （详见 docs/architecture/module-resolution/module-migrations.yaml：WebKitCache 旧路径记录 +
    // 运行 vite 返回 date=2026-09-23 22:59 的陈旧 main.ts）。
    // 重载时 webview 执行旧模块 → 旧相对 import → 命中已删除路径 → Vite ENOENT。
    // 原 Cache-Control: no-cache 仅要求 revalidate，WebKitGTK 仍会服务陈旧响应。改为 no-store：
    // dev 模块永不持久化，每次从 dev server 取最新，彻底消除「BUILD PASS ≠ DEV RUNTIME PASS」盲区。
    headers: { "Cache-Control": "no-store" },
  },
  build: {
    // WebView2 / WebKitGTK modern desktop baseline: avoid unnecessary ES2020
    // downlevel helpers for modern JS syntax. This reduces shipped bytes rather
    // than changing the frozen M0 growth threshold or metrics baseline.
    target: "es2022",
    // M0-4.b：原先全部打进单个 503 kB 的入口 chunk（触发 Vite 500 kB 告警）。
    // 按依赖来源拆分：xterm 体积最大且只在终端视图用到，单独成 chunk 后
    // 入口 chunk 显著下降，且便于后续按视图懒加载。
    rollupOptions: {
      output: {
        manualChunks(id) {
          // Browser 的 public.ts 会同时被外壳和多个懒加载能力消费。若让 Rollup
          // 自动把 public 再导出与各 Store 拆到不同 chunk，会形成 public ↔ store
          // 的跨 chunk 循环。公共出口与其状态实现必须作为同一运行时单元发布。
          if (
            id.includes("/src/capabilities/browser/public.ts") ||
            id.includes("/src/capabilities/browser/state/")
          ) {
            return "browser-runtime";
          }
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
          return undefined;
        },
      },
    },
  },
});
