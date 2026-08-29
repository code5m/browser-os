import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import "./styles/global.css";
import { bridge } from "./bridge";

// 存活标记：webview 一加载就打到后端终端，用于确认前端跑的是哪份代码。
// 改这行的时间戳即可验证"看到的 UI 是不是最新代码"。
bridge.debugLog("FE alive marker=2026-08-18-r2");

createApp(App).use(createPinia()).mount("#app");

// M0-0.b ready 钩子（契约 logs/m0-baseline-contract-v1.md §6.1）：
// 主前端完成 mount 后，再等 2 次 animation frame（首帧渲染完成），
// 然后发起一次轻量 IPC 往返（m0_ready）。后端写入带 run_id 的 ready 信号，
// 采集脚本据此记录 startup_ready_ms。非 Tauri 环境/非测量运行均静默跳过。
try {
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      bridge.m0Ready().catch(() => {});
    });
  });
} catch {
  /* 非浏览器/非 Tauri 环境忽略 */
}
