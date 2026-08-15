<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { bridge } from "./bridge";
import { useBrowserStore } from "./stores/useBrowserStore";
import { useWorkspaceStore } from "./stores/useWorkspaceStore";
import { useSystemStore } from "./stores/useSystemStore";
import { useLayoutStore } from "./stores/useLayoutStore";

import TitleBar from "./components/layout/TitleBar.vue";
import ActivityBar from "./components/layout/ActivityBar.vue";
import MainArea from "./components/layout/MainArea.vue";
import StatusBar from "./components/layout/StatusBar.vue";
import ConfirmModal from "./components/shared/ConfirmModal.vue";
import AINavPanel from "./components/browser/AINavPanel.vue";

const browser = useBrowserStore();
const ws = useWorkspaceStore();
const system = useSystemStore();
const layout = useLayoutStore();

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
  // 子 webview 右键"打开终端"：切到终端视图（TerminalPane 挂载后自动启动 shell）
  bridge.onOpenTerminal(() => {
    layout.setView("term");
    layout.showToast("💻 已打开终端");
  });
  // 子 webview 右键"选区存 Markdown 笔记"保存成功
  bridge.onNoteSaved((p) => layout.showToast("📝 笔记已保存: " + p));

  window.addEventListener("beforeunload", () => bridge.closeBrowser().catch(() => {}));
  window.addEventListener("click", ws.closeCtx);
  window.addEventListener("scroll", ws.closeCtx, true);
});
</script>

<template>
  <div class="app" :style="{ height: appHeight }">
    <TitleBar />
    <ActivityBar />
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
