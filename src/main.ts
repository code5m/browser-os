import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import "./styles/global.css";
import { bridge } from "./bridge";

// 存活标记：webview 一加载就打到后端终端，用于确认前端跑的是哪份代码。
// 改这行的时间戳即可验证"看到的 UI 是不是最新代码"。
bridge.debugLog("FE alive marker=2026-08-18-r2");

createApp(App).use(createPinia()).mount("#app");
