import { defineStore } from "pinia";
import { ref } from "vue";

// 活动栏 9 个模块 + 浏览器/编辑器，对应 prototype.html 的 9 视图 + 浏览器主视图
export type MainView =
  | "home"
  | "browser"
  | "files"
  | "clip"
  | "arts"
  | "grid"
  | "apps"
  | "term"
  | "repo"
  | "audit"
  | "editor";

export const useLayoutStore = defineStore("layout", () => {
  const mainView = ref<MainView>("browser");
  const sidebarOpen = ref(true);
  const sidebarWidth = ref(300);
  const leftTab = ref<"files" | "artifacts">("files");
  const gridToolbarOpen = ref(false);
  const clipOpen = ref(false);
  const aiNavOpen = ref(false);
  const fileEditorOpen = ref(false); // 文件编辑器/Markdown 预览覆盖层
  const msg = ref("");
  const leftResizing = ref(false);
  // 浏览器视图右侧 Dock：边浏览网页边操作文件/终端
  const browserDockOpen = ref(false);
  const browserDockTab = ref<"files" | "term">("files");
  // 地址栏模式：🌐网址（默认）/ 📁目录（输入本地路径浏览目录）
  const addrMode = ref<"url" | "dir">("url");

  let toastTimer: number | null = null;
  function showToast(text: string) {
    msg.value = text;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => {
      if (msg.value === text) msg.value = "";
    }, 2000);
  }

  // 当前是否浏览器类视图（显示地址栏 + 页签 + 视口）
  function isBrowserView() {
    return mainView.value === "browser" || mainView.value === "grid";
  }

  // 仅做 UI 状态切换；需要触发 bridge 的地方委托给其他 store 的 action
  function setView(v: MainView) {
    mainView.value = v;
    // 进入模块视图时关闭文件编辑器覆盖层
    if (v !== "editor") fileEditorOpen.value = false;
  }

  function toggleSidebar() {
    sidebarOpen.value = !sidebarOpen.value;
  }

  function toggleClipboard() {
    clipOpen.value = !clipOpen.value;
    if (clipOpen.value) setView("clip");
  }

  function toggleGridToolbar() {
    gridToolbarOpen.value = !gridToolbarOpen.value;
    const browser = useBrowserStore();
    if (gridToolbarOpen.value && !browser.gridOpen) browser.buildGrid();
    if (!gridToolbarOpen.value && browser.gridOpen) browser.closeGridAll();
  }

  function setSidebarWidth(w: number) {
    sidebarWidth.value = Math.min(560, Math.max(180, w));
  }

  // 切换地址栏模式（🌐网址 / 📁目录）
  function toggleAddrMode() {
    addrMode.value = addrMode.value === "url" ? "dir" : "url";
  }

  // 浏览器视图右侧 Dock：指定 tab 时做"切到该 tab/再点收起"语义
  function toggleBrowserDock(tab?: "files" | "term") {
    if (tab) {
      if (browserDockOpen.value && browserDockTab.value === tab) {
        browserDockOpen.value = false;
        return;
      }
      browserDockTab.value = tab;
      browserDockOpen.value = true;
    } else {
      browserDockOpen.value = !browserDockOpen.value;
    }
  }

  return {
    mainView,
    sidebarOpen,
    sidebarWidth,
    leftTab,
    gridToolbarOpen,
    clipOpen,
    aiNavOpen,
    fileEditorOpen,
    msg,
    leftResizing,
    browserDockOpen,
    browserDockTab,
    addrMode,
    showToast,
    isBrowserView,
    setView,
    toggleSidebar,
    toggleClipboard,
    toggleGridToolbar,
    setSidebarWidth,
    toggleBrowserDock,
    toggleAddrMode,
  };
});
