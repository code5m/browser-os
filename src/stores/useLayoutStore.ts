import { defineStore } from "pinia";
import { ref } from "vue";

// 活动栏 9 个模块 + 浏览器/编辑器，对应 prototype.html 的 9 视图 + 浏览器主视图
export type MainView =
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
    showToast,
    isBrowserView,
    setView,
    toggleSidebar,
    toggleClipboard,
    toggleGridToolbar,
    setSidebarWidth,
  };
});
