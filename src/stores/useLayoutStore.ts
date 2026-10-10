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
  | "capability-manager"
  | "vault";

// ===== M5-W17（Lane A6）客户端导航契约 =====
// ActivityBar 的唯一真源：一级入口、☰ 菜单分节、窄窗口密度、键盘漫游索引。
// 设计约束（由 scripts/check-client-navigation-logic.mjs 断言）：
//   1) 窄窗口从一级入口尾部裁剪，被裁掉的入口必须仍在 ☰ 菜单中可达；
//   2) 激活态只由 mainView 决定，单一判定函数 isNavActive，避免多处各写一份 ===；
//   3) 全部为纯函数，可在 Node 下直接加载测试，不触碰 bridge/后端。
// 双核心是唯一默认一级入口：更多能力仍在菜单 / Contribution 中可达。
export const TOP_NAV_ITEMS = [
  { view: "browser", icon: "🌐", label: "浏览" },
  { view: "files", icon: "📁", label: "文件" },
] as const;

export const NAV_MENU_SECTIONS = [
  {
    title: "工作区",
    items: [
      { view: "home", icon: "🏠", label: "主页" },
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
      { view: "settings", icon: "⚙️", label: "设置" },
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

// 双核心不会被宽度裁剪；低频能力在菜单中，宽度只决定导航标签密度。
export function navTopViewsForWidth(_width: number): string[] {
  return TOP_NAV_ITEMS.map((i) => i.view);
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
  const fileEditorOpen = ref(false); // 文件编辑器/Markdown 预览覆盖层
  const msg = ref("");
  const leftResizing = ref(false);
  // 浏览器视图右侧 Dock：边浏览网页边操作文件/终端
  const browserDockOpen = ref(false);
  // UI-4：Dock 页签由**能力贡献**动态提供（见 ContributionRegistry.getDockTabContributions），
  // 因此这里不再硬编码 files/term/net/session 联合类型 —— 那是把「有哪些 Dock 面板」
  // 这份业务知识放进了 Shell。Shell 只持「当前选中哪个 view」这一份 UI 选择状态。
  // owner 不变（仍为 useLayoutStore.toggleBrowserDock），语义真源唯一。
  const browserDockTab = ref<string>("files");
  // 地址栏模式：🌐网址（默认）/ 📁目录（输入本地路径浏览目录）
  const addrMode = ref<"url" | "dir">("url");
  // 三档自适应外壳：标准、紧凑、沉浸。只保存 UI 意图，不接管原生窗口生命周期。
  const shellMode = ref<"standard" | "compact" | "immersive">("standard");
  // 历史只读兼容：旧 MainArea 的“精简模式”现在专指沉浸状态。
  const compactMode = computed(() => shellMode.value === "immersive");
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
  // Phase 1：这是 mainView 的**唯一**写入点；组件禁止直接写 layout.mainView。
  function setView(v: MainView) {
    mainView.value = v;
    // 进入模块视图时关闭文件编辑器覆盖层
    if (v !== "editor") fileEditorOpen.value = false;
    // M5-W17：视图一变就收起活动条扩展行，避免"换了视图还挂着上一视图的菜单"
    navSection.value = "";
  }

  // ===== Phase 1 canonical View Intent API（View Navigation owner）=====
  // 一个用户意图 = 一个语义化入口；组件只表达意图，不拼 mainView，不做生命周期决策。
  // 这些是**纯视图导航**意图，不含 Grid 资源生命周期（后者归 useBrowserStore）。
  function activateView(v: MainView) { setView(v); }
  function activateBrowser() { setView("browser"); }
  function activateHome() { setView("home"); }
  function activateFiles() { setView("files"); }
  function activateTerm() { setView("term"); }
  function activateEditor() { setView("editor"); }
  // 语义别名：Workspace = 文件工作区视图
  function activateWorkspace() { setView("files"); }

  function toggleSidebar() {
    sidebarOpen.value = !sidebarOpen.value;
  }

  function toggleClipboard() {
    clipOpen.value = !clipOpen.value;
    if (clipOpen.value) setView("clip");
  }

  // ===== H-G RELEASE BLOCKER 修复：Shell 只持有 UI 偏好，不做资源生命周期 =====
  //
  // 旧实现在此处直接 `useBrowserStore()` 并调 openGrid()/closeGrid()，
  // 造成两个真实架构缺陷：
  //   PROBLEM A：Framework Core（本 store）反向依赖 Browser Capability 内部实现；
  //   PROBLEM B：把「持久化 UI 偏好 gridToolbarOpen」当成「立刻创建 Grid 重资源」的许可。
  //
  // 另注：该旧实现还存在一处潜伏缺陷 —— 本文件从未 import useBrowserStore，
  // 只要 toggleGridToolbar() 被调用就会抛 ReferenceError（死代码故未暴露）。
  //
  // 纠正后：gridToolbarOpen 只是「当相关 Capability 可用时，用户希望该 toolbar
  // surface 处于某状态」的**纯 UI 偏好**；是否真的创建/销毁 Grid 重资源，
  // 由 Browser Capability 自己按 availability + activation 决定（见
  // useBrowserStore 中对该偏好的单向 watch，且先过 isBrowserResourceAllowed 闸）。
  //
  // 本文件因此不再 import / 调用任何 Browser 内部（LEGACY_COUPLING → 已解除）。
  function toggleGridToolbar() {
    gridToolbarOpen.value = !gridToolbarOpen.value;
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

  function setShellMode(mode: "standard" | "compact" | "immersive") {
    if (shellMode.value === mode) return;
    shellMode.value = mode;
    navSection.value = "";
  }

  // 历史按钮和外部贡献的兼容入口；所有视图都可返回标准界面。
  function toggleCompact() {
    setShellMode(shellMode.value === "immersive" ? "standard" : "immersive");
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
  function openDirTab(path: string, reuseCurrent = false) {
    // Entering a second path in an already active DIRECTORY tab navigates it,
    // while context-menu "Open in new tab" keeps its existing deduped semantics.
    if (reuseCurrent && mainView.value === "files") {
      const current = modTabs.find((t) => t.id === activeModTab.value && !!t.path);
      if (current) {
        current.path = path;
        current.label = path.replace(/\/+$/, "").split("/").pop() || path;
        setView("files");
        return;
      }
    }
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
      import("../capabilities/browser/public").then(({ useBrowserStore }) => {
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
  function toggleBrowserDock(tab?: string) {
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
    fileEditorOpen,
    msg,
    leftResizing,
    browserDockOpen,
    browserDockTab,
    addrMode,
    shellMode,
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
    // Phase 1 canonical View Intent API
    activateView,
    activateBrowser,
    activateHome,
    activateFiles,
    activateTerm,
    activateEditor,
    activateWorkspace,
    toggleSidebar,
    toggleClipboard,
    toggleGridToolbar,
    setSidebarWidth,
    setFileTreeWidth,
    toggleBrowserDock,
    toggleAddrMode,
    setShellMode,
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
