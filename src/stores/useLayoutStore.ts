import { defineStore } from "pinia";
import { ref, reactive, computed } from "vue";

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
  | "plugin"
  | "editor"
  | "settings"
  | "vault";

// ===== M5-W17（Lane A6）客户端导航契约 =====
// ActivityBar 的唯一真源：一级入口、☰ 菜单分节、窄窗口密度、键盘漫游索引。
// 设计约束（由 scripts/check-client-navigation-logic.mjs 断言）：
//   1) 窄窗口从一级入口尾部裁剪，被裁掉的入口必须仍在 ☰ 菜单中可达；
//   2) 激活态只由 mainView 决定，单一判定函数 isNavActive，避免多处各写一份 ===；
//   3) 全部为纯函数，可在 Node 下直接加载测试，不触碰 bridge/后端。
export const TOP_NAV_ITEMS = [
  { view: "home", icon: "🏠", label: "主页" },
  { view: "browser", icon: "📁", label: "浏览" },
  { view: "term", icon: "💻", label: "终端" },
  { view: "clip", icon: "📋", label: "剪贴板" },
  { view: "arts", icon: "📚", label: "知识库" },
] as const;

export const NAV_MENU_SECTIONS = [
  {
    title: "工作区",
    items: [
      { view: "files", icon: "📂", label: "文件" },
      { view: "vault", icon: "◇", label: "笔记 Vault" },
      { view: "clip", icon: "📋", label: "剪贴板" },
      { view: "arts", icon: "📚", label: "知识库" },
    ],
  },
  {
    title: "工具",
    items: [
      // 终端既是窄窗口下会被裁掉的一级入口，也必须常驻 ☰ 菜单（否则窄窗口下不可达）
      { view: "term", icon: "💻", label: "终端" },
      { view: "apps", icon: "🚀", label: "应用" },
      { view: "scripts", icon: "📜", label: "脚本库" },
      { view: "commands", icon: "⚡", label: "命令库" },
      { view: "tools", icon: "🧰", label: "工具箱" },
      { view: "db", icon: "🗄️", label: "数据库" },
      { view: "tasks", icon: "⏰", label: "定时任务" },
      { view: "skills", icon: "🛠️", label: "技能" },
      { view: "agents", icon: "🤖", label: "智能体" },
      { view: "graph", icon: "🕸️", label: "图谱" },
      { view: "plugin", icon: "🔌", label: "插件" },
    ],
  },
  {
    title: "同步",
    items: [
      { view: "repo", icon: "🛰️", label: "仓库" },
      { view: "audit", icon: "🛡️", label: "审计" },
    ],
  },
] as const;

export const NAV_DENSITY_FULL_PX = 1180;
export const NAV_DENSITY_COMPACT_PX = 900;
export type NavDensity = "full" | "compact" | "icon";
export type NavSection = "" | "grid" | "more" | "omni";

export function navDensityForWidth(width: number): NavDensity {
  const w = Number(width) || 0;
  if (w >= NAV_DENSITY_FULL_PX) return "full";
  if (w >= NAV_DENSITY_COMPACT_PX) return "compact";
  return "icon";
}

// 窄窗口裁剪：full 全显示 → compact 保留前 3 个 → icon 只留主页+浏览。
// 被裁掉的（终端/剪贴板/知识库）在 NAV_MENU_SECTIONS 中仍然可达。
export function navTopViewsForWidth(width: number): string[] {
  const d = navDensityForWidth(width);
  if (d === "full") return TOP_NAV_ITEMS.map((i) => i.view);
  if (d === "compact") return TOP_NAV_ITEMS.slice(0, 3).map((i) => i.view);
  return TOP_NAV_ITEMS.slice(0, 2).map((i) => i.view);
}

// 激活态唯一判定：只有当前视图与入口视图完全一致才算激活（不留"包含/前缀"歧义）
export function isNavActive(mainView: string, view: string): boolean {
  return mainView === view;
}

// 左右方向键漫游（含首尾环绕）；非法长度返回 0，绝不抛异常
export function nextNavIndex(current: number, delta: number, len: number): number {
  if (!Number.isFinite(len) || len <= 0) return 0;
  const c = Number.isFinite(current) ? current : 0;
  return (((c + delta) % len) + len) % len;
}

// 模块页签元数据：W17 起提到模块级导出，供 check-client-navigation-logic.mjs
// 断言"每个模块都有导航入口，不存在孤儿模块"。
export const MODULE_META: Record<string, { icon: string; label: string }> = {
  home: { icon: "🏠", label: "主页" },
  grid: { icon: "🗂️", label: "宫格" },
  files: { icon: "📂", label: "文件" },
  vault: { icon: "◇", label: "笔记 Vault" },
  db: { icon: "▤", label: "数据库" },
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
  plugin: { icon: "🔌", label: "插件" },
  settings: { icon: "⚙️", label: "设置" },
};

export const useLayoutStore = defineStore("layout", () => {
  const mainView = ref<MainView>("home");
  const sidebarOpen = ref(true);
  const sidebarWidth = ref(300);
  // 文件夹目录树（FilePanel .ftree）宽度，可由鼠标拖拽调整
  const fileTreeWidth = ref(260);
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
  // M5-W17：客户端窗口宽度（px）→ 导航密度；ActivityBar 随 resize 上报
  const windowWidth = ref(NAV_DENSITY_FULL_PX);
  // M5-W17：活动条扩展行（宫格设置 / ☰ 菜单 / 最近与常用），同一时刻只开一个
  const navSection = ref<NavSection>("");

  const navDensity = computed<NavDensity>(() => navDensityForWidth(windowWidth.value));
  const navTopViews = computed<string[]>(() => navTopViewsForWidth(windowWidth.value));

  function setWindowWidth(px: number) {
    const n = Math.round(Number(px) || 0);
    windowWidth.value = Math.min(4096, Math.max(320, n));
  }

  function toggleNavSection(key: NavSection) {
    navSection.value = navSection.value === key ? "" : key;
  }

  function closeNavSection() {
    navSection.value = "";
  }

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
    // M5-W17：视图一变就收起活动条扩展行，避免"换了视图还挂着上一视图的菜单"
    navSection.value = "";
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

  function setFileTreeWidth(w: number) {
    fileTreeWidth.value = Math.min(560, Math.max(160, w));
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
  const MOD_META: Record<string, { icon: string; label: string }> = MODULE_META;
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
    fileTreeWidth,
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
    windowWidth,
    navSection,
    navDensity,
    navTopViews,
    setWindowWidth,
    toggleNavSection,
    closeNavSection,
    showToast,
    isBrowserView,
    setView,
    toggleSidebar,
    toggleClipboard,
    toggleGridToolbar,
    setSidebarWidth,
    setFileTreeWidth,
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
