import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import "./styles/tokens.css";
import "./styles/global.css";
import { bridge } from "./bridge";
import { bootstrapCapabilityRuntimeAsync, vaultPorts, clipboardPorts } from "./capability";
import { VAULT_PORTS_KEY } from "@browser-os/capability-vault";
import { CLIPBOARD_PORTS_KEY } from "@browser-os/capability-clipboard";

document.documentElement.style.background = "#fff";
document.body.style.background = "#fff";

// Production startup awaits each Owner's transition before mounting its UI.
// Startup errors are isolated: a failed Capability must never suppress the shell.
// 存活标记：webview 一加载就打到后端终端，用于确认前端跑的是哪份代码。
// 改这行的时间戳即可验证"看到的 UI 是不是最新代码"。
bridge.debugLog("FE alive marker=2026-09-13-install-gui-r2");

function probeFirstPaint() {
  requestAnimationFrame(() => {
    const rootEl = document.querySelector("#app") as HTMLElement | null;
    const appEl = document.querySelector(".app") as HTMLElement | null;
    const appRect = appEl?.getBoundingClientRect();
    const bodyRect = document.body.getBoundingClientRect();
    bridge.debugLog(
      `[FE] first-paint probe readyState=${document.readyState} root=${rootEl ? "ok" : "missing"} body=${Math.round(
        bodyRect.width
      )}x${Math.round(bodyRect.height)} app=${Math.round(appRect?.width ?? 0)}x${Math.round(
        appRect?.height ?? 0
      )} bodyBg=${getComputedStyle(document.body).backgroundColor} appBg=${
        appEl ? getComputedStyle(appEl).backgroundColor : "missing"
      } text=${JSON.stringify((appEl?.textContent ?? "").slice(0, 80))}`
    );
  });
}

function mountApp() {
  const rootEl = document.querySelector("#app");
  if (!rootEl) {
    bridge.debugLog(`[FE] #app missing at ${document.readyState}, waiting DOMContentLoaded`);
    window.addEventListener("DOMContentLoaded", mountApp, { once: true });
    return;
  }
  try {
    const app = createApp(App);
    app.config.errorHandler = (error, _instance, info) => {
      bridge.debugLog(`[FE] Vue error ${info}: ${error instanceof Error ? error.stack || error.message : String(error)}`);
    };
    app.use(createPinia());
    // Vault M2：注入 Host 表现层契约（窄 port），供包内组件经 inject 取用，不反向依赖 Host。
    // provide 必须在 mount 之前调用，否则已挂载组件树 inject 不到。
    app.provide(VAULT_PORTS_KEY, vaultPorts);
    app.provide(CLIPBOARD_PORTS_KEY, clipboardPorts);
    app.mount(rootEl);
    bridge.debugLog("[FE] vue mounted");
  } catch (e) {
    bridge.debugLog("[FE] vue mount failed: " + (e instanceof Error ? e.message : String(e)));
  }
  probeFirstPaint();
}

// M0-0.b ready 钩子（契约 logs/m0-baseline-contract-v1.md §6.1）：
// 主前端完成 mount 后，再等 2 次 animation frame（首帧渲染完成），
// 然后发起一次轻量 IPC 往返（m0_ready）。后端写入带 run_id 的 ready 信号，
// 采集脚本据此记录 startup_ready_ms。非 Tauri 环境/非测量运行均静默跳过。
function signalFirstPaintReady() {
try {
  requestAnimationFrame(() => {
    bridge.debugLog("[M0] rAF1 fired");
    requestAnimationFrame(() => {
      bridge.debugLog("[M0] rAF2 fired, invoking m0_ready");
      bridge
        .m0Ready()
        .then((rid) => bridge.debugLog("[M0] m0_ready ok run_id=" + rid))
        .catch((e) => bridge.debugLog("[M0] m0_ready err: " + (e ?? "unknown")));
    });
  });
} catch {
  /* 非浏览器/非 Tauri 环境忽略 */
}
}

// Publish the runtime gate synchronously, before Vue components read capability state.
// Never wait for slow async Owner hooks before the first paint.
const capabilityBootstrap = bootstrapCapabilityRuntimeAsync();
mountApp();
signalFirstPaintReady();
void capabilityBootstrap
  .then(boot => bridge.debugLog(`[capability] async bootstrap activated=${boot.activated} error=${boot.error ?? "none"}`))
  .catch(error => bridge.debugLog(`[capability] async bootstrap failed: ${String(error)}`));
