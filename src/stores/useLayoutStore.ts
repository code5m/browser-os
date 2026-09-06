import { defineStore } from "pinia";
import { ref, reactive } from "vue";

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
  | "scripts"
  | "commands"
  | "tools"
  | "db"
  | "tasks"
  | "skills"
  | "agents"
  | "graph"
  | "editor";

export const useLayoutStore = defineStore("layout", () => {
  const mainView = ref<MainView>("home");
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
  const browserDockTab = ref<"files" | "term" | "net" | "session">("files");
  // 地址栏模式：🌐网址（默认）/ 📁目录（输入本地路径浏览目录）
  const addrMode = ref<"url" | "dir">("url");
  // 浏览器精简模式：隐藏地址栏+页签栏，给网页更大空间（类谷歌沉浸式）
  const compactMode = ref(false);

  let toastTimer: number | null = null;
  function showToast(text: string) {
    msg.value = text;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => {
      if (msg.value === text) msg.value = "";
    }, 4000);
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

  // 切换浏览器精简模式
  function toggleCompact() {
    compactMode.value = !compactMode.value;
  }

  // ===== 模块页签：菜单功能与浏览器一致 —— 点一个就新建一个标签 =====
  interface ModTab {
    id: string;
    view: MainView;
    icon: string;
    label: string;
    path?: string; // 目录页签：记录的目录路径，激活时重新 enterDir
  }
  const MOD_META: Record<string, { icon: string; label: string }> = {
    home: { icon: "🏠", label: "主页" },
    grid: { icon: "🗂️", label: "宫格" },
    files: { icon: "📂", label: "文件" },
    clip: { icon: "📋", label: "剪贴板" },
    arts: { icon: "📚", label: "知识库" },
    apps: { icon: "🚀", label: "应用" },
    term: { icon: "💻", label: "终端" },
    repo: { icon: "🛰️", label: "仓库" },
    audit: { icon: "🛡️", label: "审计" },
    scripts: { icon: "📜", label: "脚本库" },
    commands: { icon: "⚡", label: "命令库" },
    tools: { icon: "🧰", label: "工具箱" },
    tasks: { icon: "⏰", label: "定时任务" },
    skills: { icon: "🛠️", label: "技能" },
    agents: { icon: "🤖", label: "智能体" },
    graph: { icon: "🕸️", label: "图谱" },
    settings: { icon: "⚙️", label: "设置" },
  };
  const modTabs = reactive<ModTab[]>([]);
  const activeModTab = ref("");
  let modTabSeq = 0;

  function isModView() {
    return mainView.value in MOD_META;
  }

  // 同视图去重复用：已存在则激活，否则新建（避免主页/文件等同一模块被反复点开成一排）
  function openModule(view: MainView) {
    const meta = MOD_META[view] ?? { icon: "📄", label: view };
    const existing = modTabs.find((t) => t.view === view);
    if (existing) {
      activeModTab.value = existing.id;
      setView(existing.view);
      return;
    }
    const id = `mod-${++modTabSeq}`;
    modTabs.push({ id, view, ...meta });
    activeModTab.value = id;
    setView(view);
  }

  // 目录页签：同路径去重复用（避免一排完全相同的目录标签），新路径新建
  function openDirTab(path: string) {
    const existing = modTabs.find((t) => t.path === path);
    if (existing) {
      activeModTab.value = existing.id;
      setView(existing.view);
      return;
    }
    const name = path.replace(/\/+$/, "").split("/").pop() || path;
    const id = `dir-${++modTabSeq}`;
    modTabs.push({ id, view: "files", icon: "📁", label: name, path });
    activeModTab.value = id;
    setView("files");
  }

  function activateModTab(id: string) {
    const t = modTabs.find((x) => x.id === id);
    if (!t) return;
    activeModTab.value = id;
    setView(t.view);
    // 目录页签：地址栏同步显示目录路径。
    // M0-4.b：此处**必须**保持动态引入——useBrowserStore 深度依赖本 store
    // （mainView / showToast / isBrowserView 等多处），改为静态会形成
    // layout ↔ browser 循环依赖。Vite 的 mix 告警属已知且必要的取舍，
    // 已在 checkpoint `M0-4.b` 记录，不要为了消除告警把它改成静态引入。
    if (t.path) {
      const p = t.path;
      import("./useBrowserStore").then(({ useBrowserStore }) => {
        useBrowserStore().url = p;
      });
    }
  }

  function closeModTab(id: string) {
    const i = modTabs.findIndex((x) => x.id === id);
    if (i < 0) return;
    const wasActive = activeModTab.value === id;
    modTabs.splice(i, 1);
    if (!wasActive) return;
    const next = modTabs[i] || modTabs[i - 1];
    if (next) {
      activeModTab.value = next.id;
      setView(next.view);
    } else {
      activeModTab.value = "";
      setView("home");
    }
  }

  // 浏览器视图右侧 Dock：指定 tab 时做"切到该 tab/再点收起"语义
  function toggleBrowserDock(tab?: "files" | "term" | "net" | "session") {
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
    compactMode,
    showToast,
    isBrowserView,
    setView,
    toggleSidebar,
    toggleClipboard,
    toggleGridToolbar,
    setSidebarWidth,
    toggleBrowserDock,
    toggleAddrMode,
    toggleCompact,
    modTabs,
    activeModTab,
    isModView,
    openModule,
    openDirTab,
    activateModTab,
    closeModTab,
  };
});
