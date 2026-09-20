<script setup lang="ts">
import { onMounted, onErrorCaptured, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { bridge } from "./bridge";
import { useBrowserStore } from "./capabilities/browser/public";
import { useResourceStore } from "./stores/useResourceStore";
import { useSessionStore } from "./stores/useSessionStore";
import { useWorkspaceStore } from "./capabilities/workspace/public";
import { useArtifactStore } from "./capabilities/workspace/public";
import { useRepoStore } from "./capabilities/workspace/public";
import { useFileStore } from "./capabilities/workspace/public";
import { useGitStore } from "./stores/useGitStore";
import { useSystemStore } from "./stores/useSystemStore";
import { useLayoutStore } from "./stores/useLayoutStore";
import { useSettingsStore } from "./stores/useSettingsStore";

import ActivityBar from "./components/layout/ActivityBar.vue";
import UnifiedTabBar from './components/layout/UnifiedTabBar.vue';
import WorkbenchCommands from './components/layout/WorkbenchCommands.vue';
import WorkbenchRail from './components/layout/WorkbenchRail.vue';
import MainArea from "./components/layout/MainArea.vue";
import StatusBar from "./components/layout/StatusBar.vue";
import ConfirmModal from "./components/shared/ConfirmModal.vue";
import GitWriteConfirmDialog from "./components/workspace/GitWriteConfirmDialog.vue";
import ImageLightbox from "./components/shared/ImageLightbox.vue";
import AINavPanel from "./components/browser/AINavPanel.vue";

const browser = useBrowserStore();
const resources = useResourceStore();
const session = useSessionStore();
const ws = useWorkspaceStore();
const art = useArtifactStore();
const rp = useRepoStore();
const fs = useFileStore();
const git = useGitStore();
const system = useSystemStore();
const layout = useLayoutStore();
const settings = useSettingsStore();

// W17(A7): 外壳级兜底状态——启动遮罩与渲染错误兜底（纯展示，不引入运行时行为）。
const ready = ref(false);
const shellError = ref<string | null>(null);

// W17(A7): 捕获子树渲染异常，避免整树白屏/死区；仅记录通用提示，不暴露内部错误细节。
onErrorCaptured((err: unknown) => {
  // eslint-disable-next-line no-console
  console.error("[app] render error captured:", (err as Error)?.message ?? err);
  shellError.value = "界面渲染遇到问题，请重启客户端。";
  return false;
});

const appHeight = ref<string>("100vh");

async function syncWindowSize() {
  try {
    const size = await getCurrentWindow().innerSize();
    appHeight.value = `${size.height / (await getCurrentWindow().scaleFactor())}px`;
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
  ready.value = true;
  try {
  if (!(window as any).__TAURI_INTERNALS__) {
    return;
  }
  await syncWindowSize();
  const unlisten = await getCurrentWindow().onResized(syncWindowSize);
  window.addEventListener("beforeunload", unlisten);
  // M0-0.b 终端吞吐（契约 §6.3）：测量模式下自动挂载终端面板（前端驱动 10 MiB 负载）
  system.loadM0Config().then(() => {
    if (system.m0Cfg?.driver === "term-throughput") {
      layout.activateTerm();
    }
  });
  ws.loadRecents();
  system.loadClipHistory();
  system.startClipWatch();
  await ws.refresh();
  await fs.loadStartDirs();
  if (fs.startDirs.length > 0) {
    const wsDir = fs.startDirs.find((d) => d.name.includes("成果工作区"));
    fs.enterDir(wsDir ? wsDir.path : fs.startDirs[0].path);
  }

  bridge.onSyncCompleted((j) => rp.onSyncCompleted(j));
  // M1-7：Git 写任务完成（成功/失败）→ 刷新 status/diff/branch；
  // 订阅放全局，保证切换视图/面板卸载后仍能收到完成事件并刷新状态。
  bridge.onGitWriteCompleted((j) => git.onWriteCompleted(j));
  bridge.onBrowserResources((r) => browser.setResources(r));
  bridge.onArtifactCollected(() => {
    ws.refresh();
    layout.showToast("✅ 已保存到成果库");
  });
  bridge.onTabTitle((t) => browser.setTitle(t));
  bridge.onTabNavigated((d) => browser.setNavigated(d.id, d.url));
  // M1-8：资源瀑布实时事件（payload 已是后端脱敏 DTO）。订阅放全局，
  // 保证 Dock 面板未挂载时记录也不丢。
  bridge.onResourceReceived((r) => resources.applyReceived(r));
  // 普通 Tab 关闭（Owner 最终裁决 2026-09-12）：不弹确认框、不持久化、直接关闭。
  // 关闭入口统一走 browser.tabClose → closeTabNow（内部写入 recentlyClosed 内存栈）。
  // 不再挂接任何关闭拦截器 / 确认框（旧的关闭确认弹窗已撤销）。
  bridge.onTabRecovery((d) => browser.handleTabRecovery(d));
  bridge.onNewTabRequest((u) => {
    setTimeout(() => browser.tabNew(u.url), 0);
  });
  // M1-4：外部打开 URL（xdg-open / 默认浏览器路由）。后端统一进 pending 队列，
  // 前端拉取即清空（天然去重）；就绪后新到的 URL 经 pending 提示触发立即拉取。
  async function drainPendingOpenUrls() {
    try {
      const urls = await bridge.takePendingOpenUrls();
      for (const url of urls) {
        layout.showToast("🌐 外部链接已打开");
        browser.tabNew(url);
      }
    } catch (e) {
      // eslint-disable-next-line no-console
      console.warn("[open-url] drain pending failed", e);
    }
  }
  bridge.onOpenUrlPending(() => {
    drainPendingOpenUrls();
  });
  bridge.onOpenUrlRejected(() => {
    layout.showToast("⚠️ 已拒绝非 http/https 链接");
  });
  bridge.onTermData((d) => system.onTermData(d));
  // 子 webview 右键"打开终端"：浏览器视图下优先开右侧 Dock（不离开网页），否则切全屏终端视图
  bridge.onOpenTerminal(() => {
    if (layout.mainView === "browser") {
      layout.toggleBrowserDock("term");
    } else {
      layout.activateTerm();
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
    if (system.m0Cfg?.driver || e.isComposing) return;
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
      else layout.activateTerm();
    } else if (matchKey(e, km.grid)) {
      e.preventDefault();
      browser.activateGrid();
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
    } else if (matchKey(e, km.recentlyClosed)) {
      e.preventDefault();
      browser.restoreRecent();
    }
  }
  window.addEventListener("keydown", onGlobalKeydown);

  window.addEventListener("beforeunload", () => bridge.closeBrowser().catch(() => {}));
  // M1-9：前端卸载前 flush（与后端 ShutdownCoordinator 的 flush-sessions 双保险）
  window.addEventListener("beforeunload", () => bridge.flushSessions().catch(() => {}));
  window.addEventListener("click", art.closeCtx);
  window.addEventListener("scroll", art.closeCtx, true);
  // 文件树右键菜单同样需要点空白/滚动时关闭
  window.addEventListener("click", fs.closeFileCtx);
  window.addEventListener("scroll", fs.closeFileCtx, true);
  // M1-4 冷启动兜底：进程启动时 argv 带入的 URL 已在后端队列，
  // 挂载完成后首次拉取（此后经 onOpenUrlPending 提示增量拉取）。
  drainPendingOpenUrls();
  } catch (e) {
    // eslint-disable-next-line no-console
    console.error("[app] startup init failed:", e);
    shellError.value = "客户端初始化遇到问题，请查看日志。";
  }
});
</script>

<template>
  <div class="app" :style="{ height: appHeight }">
    <!-- W17(A7): 启动遮罩——应用就绪前展示，避免首屏空白 -->
    <div v-if="!ready && !shellError" class="boot-overlay" role="status" aria-live="polite">
      <div class="boot-spinner" aria-hidden="true"></div>
      <div class="boot-text">正在启动…</div>
    </div>
    <!-- W17(A7): 外壳级错误兜底——子树渲染失败时不留白屏/死区 -->
    <div v-else-if="shellError" class="shell-error" role="alert">
      <div class="se-title">客户端遇到问题</div>
      <div class="se-desc">{{ shellError }}</div>
    </div>
    <template v-else>
      <!-- 精简模式：整行工具栏隐藏，网页占满（由 MainArea 的 ☰ 悬浮钮退出） -->
      <UnifiedTabBar v-if="!layout.compactMode" />
      <ActivityBar v-if="!layout.compactMode" />
      <WorkbenchCommands />
      <div class="body">
        <WorkbenchRail v-if="!layout.compactMode" />
        <AINavPanel />
        <MainArea />
      </div>
      <StatusBar />
      <ConfirmModal />
      <!-- M1-7 Git 写确认闸门：全局挂载，保证任何视图下待确认任务都能被看到/处理 -->
      <GitWriteConfirmDialog />
      <!-- 最近关闭页签恢复入口见 SessionPanel；普通关闭不再弹确认框 -->

      <!-- M2-2.b 图片灯箱：全局挂载，任何视图点开画廊都能放大预览 -->
      <ImageLightbox />
    </template>
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
.boot-overlay {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 14px;
  background: #0f172a;
  color: #cbd5e0;
  z-index: 99998;
}
.boot-spinner {
  width: 28px;
  height: 28px;
  border: 3px solid #2d3748;
  border-top-color: #5b9dff;
  border-radius: 50%;
  animation: boot-spin 0.8s linear infinite;
}
@keyframes boot-spin {
  to { transform: rotate(360deg); }
}
.boot-text {
  font-size: 14px;
}
.shell-error {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  background: #0f172a;
  color: #ffb4b4;
  z-index: 99998;
  padding: 24px;
  text-align: center;
}
.se-title {
  font-size: 16px;
  font-weight: 600;
}
.se-desc {
  font-size: 14px;
  color: #cbd5e0;
  max-width: 480px;
  line-height: 1.7;
}
</style>
