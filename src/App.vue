<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { bridge } from "./bridge";
import { useBrowserStore } from "./stores/useBrowserStore";
import { useWorkspaceStore } from "./stores/useWorkspaceStore";
import { useSystemStore } from "./stores/useSystemStore";
import { useLayoutStore } from "./stores/useLayoutStore";
import { useSettingsStore } from "./stores/useSettingsStore";

import ActivityBar from "./components/layout/ActivityBar.vue";
import MainArea from "./components/layout/MainArea.vue";
import StatusBar from "./components/layout/StatusBar.vue";
import ConfirmModal from "./components/shared/ConfirmModal.vue";
import AINavPanel from "./components/browser/AINavPanel.vue";

const browser = useBrowserStore();
const ws = useWorkspaceStore();
const system = useSystemStore();
const layout = useLayoutStore();
const settings = useSettingsStore();

const appHeight = ref<string>("100vh");

async function syncWindowSize() {
  try {
    const size = await getCurrentWindow().innerSize();
    appHeight.value = `${size.height}px`;
    // eslint-disable-next-line no-console
    console.log(
      "[app] tauri innerSize=",
      size,
      "window.innerHeight=",
      window.innerHeight,
      "devicePixelRatio=",
      window.devicePixelRatio,
      "appHeight=",
      appHeight.value
    );
  } catch (e) {
    appHeight.value = "100vh";
    // eslint-disable-next-line no-console
    console.log("[app] innerSize error", e);
  }
}

onMounted(async () => {
  await syncWindowSize();
  const unlisten = await getCurrentWindow().onResized(syncWindowSize);
  window.addEventListener("beforeunload", unlisten);
  ws.loadRecents();
  system.loadClipHistory();
  system.startClipWatch();
  await ws.refresh();
  await ws.loadStartDirs();
  if (ws.startDirs.length > 0) {
    const wsDir = ws.startDirs.find((d) => d.name.includes("成果工作区"));
    ws.enterDir(wsDir ? wsDir.path : ws.startDirs[0].path);
  }

  bridge.onSyncCompleted((j) => ws.onSyncCompleted(j));
  bridge.onBrowserResources((r) => browser.setResources(r));
  bridge.onArtifactCollected(() => {
    ws.refresh();
    layout.showToast("✅ 已保存到成果库");
  });
  bridge.onTabTitle((t) => browser.setTitle(t));
  bridge.onTabNavigated((d) => browser.setNavigated(d.id, d.url));
  bridge.onNewTabRequest((u) => {
    setTimeout(() => browser.tabNew(u.url), 0);
  });
  bridge.onTermData((d) => system.onTermData(d));
  // 子 webview 右键"打开终端"：浏览器视图下优先开右侧 Dock（不离开网页），否则切全屏终端视图
  bridge.onOpenTerminal(() => {
    if (layout.mainView === "browser") {
      layout.toggleBrowserDock("term");
    } else {
      layout.setView("term");
    }
    layout.showToast("💻 已打开终端");
  });
  // 子 webview 右键"选区存 Markdown 笔记"保存成功
  bridge.onNoteSaved((p) => layout.showToast("📝 笔记已保存: " + p));

  // ===== 全局快捷键（按当前方案匹配） =====
  function matchKey(e: KeyboardEvent, combo: string): boolean {
    const parts = combo.toLowerCase().split("+");
    const key = parts[parts.length - 1];
    const needCtrl = parts.includes("ctrl");
    const needShift = parts.includes("shift");
    const needAlt = parts.includes("alt");
    if (e.ctrlKey !== needCtrl || e.shiftKey !== needShift || e.altKey !== needAlt)
      return false;
    const ek = e.key.toLowerCase();
    if (key === "tab") return ek === "tab";
    if (key === "`") return ek === "`" || ek === "~";
    if (key.startsWith("f") && key.length <= 3) return ek === key;
    if (key === "pagedown") return ek === "pagedown";
    if (key === "pageup") return ek === "pageup";
    if (key === "right") return ek === "arrowright";
    if (key === "left") return ek === "arrowleft";
    return ek === key;
  }

  function onGlobalKeydown(e: KeyboardEvent) {
    // 输入框/文本域内不触发全局快捷键
    const t = e.target as HTMLElement;
    if (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)
      return;
    const km = settings.currentKeymap;
    if (matchKey(e, km.newTab)) {
      e.preventDefault();
      browser.tabNew();
    } else if (matchKey(e, km.closeTab)) {
      e.preventDefault();
      if (browser.activeTabId) browser.tabClose(browser.activeTabId);
    } else if (matchKey(e, km.nextTab)) {
      e.preventDefault();
      const idx = browser.tabs.findIndex((x) => x.id === browser.activeTabId);
      const next = browser.tabs[idx + 1] || browser.tabs[0];
      if (next) browser.tabSwitch(next.id);
    } else if (matchKey(e, km.prevTab)) {
      e.preventDefault();
      const idx = browser.tabs.findIndex((x) => x.id === browser.activeTabId);
      const prev = browser.tabs[idx - 1] || browser.tabs[browser.tabs.length - 1];
      if (prev) browser.tabSwitch(prev.id);
    } else if (matchKey(e, km.terminal)) {
      e.preventDefault();
      if (layout.mainView === "browser") layout.toggleBrowserDock("term");
      else layout.setView("term");
    } else if (matchKey(e, km.grid)) {
      e.preventDefault();
      layout.openModule("grid");
      browser.buildGrid();
    } else if (matchKey(e, km.home)) {
      e.preventDefault();
      layout.openModule("home");
    } else if (matchKey(e, km.reload)) {
      e.preventDefault();
      browser.reloadActive();
    } else if (matchKey(e, km.focusAddr)) {
      e.preventDefault();
      const inp = document.querySelector(".omni-wrap input") as HTMLInputElement;
      inp?.focus();
      inp?.select();
    }
  }
  window.addEventListener("keydown", onGlobalKeydown);

  window.addEventListener("beforeunload", () => bridge.closeBrowser().catch(() => {}));
  window.addEventListener("click", ws.closeCtx);
  window.addEventListener("scroll", ws.closeCtx, true);
  // 文件树右键菜单同样需要点空白/滚动时关闭
  window.addEventListener("click", ws.closeFileCtx);
  window.addEventListener("scroll", ws.closeFileCtx, true);

  // TEMP-REPRO 崩溃复现：完整复现「开宫格 → 等 AI 页面加载 → 群发 1+1=? 回车」链路
  // （闪崩发生在回车触发 gridSendAi → evalInTab 注入执行之后，gdb 抓栈用，调试完成后删除）
  window.setTimeout(async () => {
    browser.gridMode = "ai";
    layout.openModule("grid");
    await browser.buildGrid();
    // 等 4 个 AI 页面加载出输入框（豆包/通义/Kimi/DeepSeek 首屏+懒加载，给足 12s）
    window.setTimeout(async () => {
      browser.gridAiInput = "1+1=?";
      await browser.gridSendAi(); // 真正的崩溃触发点：回车 → evalInTab 注入
    }, 12000);
  }, 4000);
});
</script>

<template>
  <div class="app" :style="{ height: appHeight }">
    <!-- 精简模式：整行工具栏隐藏，网页占满（由 MainArea 的 ☰ 悬浮钮退出） -->
    <ActivityBar v-show="!layout.compactMode" />
    <div class="body">
      <AINavPanel />
      <MainArea />
    </div>
    <StatusBar />
    <ConfirmModal />
  </div>
</template>

<style>
.app {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.app .body {
  flex: 1;
  display: flex;
  min-height: 0;
}
</style>
