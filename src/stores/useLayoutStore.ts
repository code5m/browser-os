import { defineStore } from "pinia";
import { ref, reactive, computed } from "vue";
import { documentPathKey, documentPathName, sameDocumentPath, replaceDocumentPathPrefix } from "../application/documentPaths.mjs";

export type ViewChangeCommit = (() => boolean) & { isCurrent(): boolean };

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
  | "vault"
  | "bfc"
  | "wechat";

// ===== M5-W17（Lane A6）客户端导航契约 =====
// ActivityBar 的唯一真源：一级入口、☰ 菜单分节、窄窗口密度、键盘漫游索引。
// 设计约束（由 scripts/check-client-navigation-logic.mjs 断言）：
//   1) 窄窗口从一级入口尾部裁剪，被裁掉的入口必须仍在 ☰ 菜单中可达；
//   2) 激活态只由 mainView 决定，单一判定函数 isNavActive，避免多处各写一份 ===；
//   3) 全部为纯函数，可在 Node 下直接加载测试，不触碰 bridge/后端。
// 双核心是唯一默认一级入口：更多能力仍在菜单 / Contribution 中可达。
export const TOP_NAV_ITEMS = [
  { view: "home", icon: "⌂", label: "主页" },
  { view: "browser", icon: "🌐", label: "浏览" },
  { view: "files", icon: "📁", label: "文件" },
  { view: "tools", icon: "◈", label: "工具" },
] as const;

export const NAV_MENU_SECTIONS = [
  { title: "工作区", items: [
    { view: "grid", icon: "▦", label: "宫格" },
    { view: "clip", icon: "📋", label: "剪贴板" },
    { view: "apps", icon: "🚀", label: "应用" },
  ] },
  { title: "智能体中心", items: [
    { view: "agents", icon: "🤖", label: "智能体" },
    { view: "skills", icon: "⚙", label: "技能（智能体能力）" },
  ] },
  { title: "知识中心", items: [
    { view: "graph", icon: "🕸️", label: "知识图谱" },
    { view: "vault", icon: "◇", label: "笔记" },
    { view: "arts", icon: "📚", label: "知识库" },
  ] },
  { title: "开发与自动化", items: [
    { view: "term", icon: "💻", label: "终端" },
    { view: "scripts", icon: "📜", label: "脚本库" },
    { view: "commands", icon: "⚡", label: "命令库" },
    { view: "db", icon: "🗄️", label: "数据库" },
    { view: "tasks", icon: "⏰", label: "定时任务" },
  ] },
  { title: "系统与治理", items: [
    { view: "repo", icon: "🛰️", label: "仓库" },
    { view: "audit", icon: "🛡️", label: "审计" },
    { view: "plugin", icon: "🔌", label: "插件" },
    { view: "settings", icon: "⚙️", label: "设置" },
  ] },
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
  grid: { icon: "▦", label: "宫格" },
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
  bfc: { icon: "▦", label: "BFC 内容库" },
  wechat: { icon: "📣", label: "公众号工作台" },
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
  const addrMode = ref<"auto" | "url" | "dir">("auto");
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

  // A failed exit save remains visible until another exit attempt or an
  // explicit acknowledgement. Ordinary transient messages cannot hide it.
  const exitSaveNotice = ref("");
  function setExitSaveNotice(text: string) { exitSaveNotice.value = text; }

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

  // Workbench owners can block context changes that would discard unsaved edits.
  // No imports of capability internals here: an owner registers a synchronous guard.
  const beforeViewChange = new Map<string, (next: MainView, previous: MainView) => boolean>();
  function registerBeforeViewChange(key: string, callback: (next: MainView, previous: MainView) => boolean) {
    beforeViewChange.set(key, callback);
    return () => {
      if (beforeViewChange.get(key) === callback) beforeViewChange.delete(key);
    };
  }
  function canNavigateTo(v: MainView): boolean {
    return [...beforeViewChange.values()].every((guard) => guard(v, mainView.value));
  }
  // Layout is the sole writer of the visible view. An asynchronous owner may
  // reserve a guarded navigation, perform its work, then publish only if no
  // newer navigation has taken over. A rejected/failed action never changes UI.
  let navigationRevision = 0;
  // Background initialization may observe an epoch, but cannot publish a view.
  function captureNavigation(): () => boolean {
    const revision = navigationRevision;
    return () => revision === navigationRevision;
  }
  function commitView(v: MainView): boolean {
    mainView.value = v;
    navigationRevision += 1;
    if (v !== "editor") fileEditorOpen.value = false;
    navSection.value = "";
    return true;
  }
  function navigationCommit(isCurrent: () => boolean, publish: () => boolean): ViewChangeCommit {
    return Object.assign(() => isCurrent() && publish(), { isCurrent });
  }
  function prepareViewChange(v: MainView, contextChanged = false): ViewChangeCommit | null {
    if ((v !== mainView.value || contextChanged) && !canNavigateTo(v)) return null;
    // Accepting a newer intent supersedes in-flight owners even before that
    // intent has completed its directory/native work.
    const revision = ++navigationRevision;
    return navigationCommit(() => revision === navigationRevision, () => commitView(v));
  }
  function setView(v: MainView): boolean {
    const publish = prepareViewChange(v);
    return publish ? publish() : false;
  }

  // ===== Phase 1 canonical View Intent API（View Navigation owner）=====
  // 一个用户意图 = 一个语义化入口；组件只表达意图，不拼 mainView，不做生命周期决策。
  // 这些是**纯视图导航**意图，不含 Grid 资源生命周期（后者归 useBrowserStore）。
  function activateView(v: MainView) { return setView(v); }
  function activateBrowser() { return setView("browser"); }
  function activateHome() { return setView("home"); }
  function activateFiles() { return setView("files"); }
  function activateTerm() { return setView("term"); }
  function activateEditor() { return setView("editor"); }
  // 语义别名：Workspace = 文件工作区视图
  function activateWorkspace() { return setView("files"); }

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
    addrMode.value = addrMode.value === "auto" ? "url" : addrMode.value === "url" ? "dir" : "auto";
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
  function prepareModule(view: MainView): ViewChangeCommit | null {
    const existing = modTabs.find((t) => t.view === view && !t.path);
    if (existing) return prepareModTabActivation(existing.id);
    const publish = prepareViewChange(view, mainView.value === view);
    if (!publish) return null;
    return navigationCommit(publish.isCurrent, () => {
      if (!publish()) return false;
      const meta = MOD_META[view] ?? { icon: "📄", label: view };
      const id = `mod-${++modTabSeq}`;
      modTabs.push({ id, view, ...meta });
      activeModTab.value = id;
      return true;
    });
  }
  function openModule(view: MainView): boolean {
    const publish = prepareModule(view);
    return publish ? publish() : false;
  }

  // Directory identity is shared with Workspace; original paths remain intact.
  // Async callers prepare, read through Workspace, then publish this intent.
  function prepareDirTab(path: string, reuseCurrent = false): ViewChangeCommit | null {
    documentPathKey("", path);
    const name = documentPathName(path);
    if (reuseCurrent && mainView.value === "files") {
      const current = modTabs.find((t) => t.id === activeModTab.value && !!t.path);
      if (current) {
        const oldPath = current.path!;
        const publish = prepareViewChange("files", !sameDocumentPath(oldPath, path));
        if (!publish) return null;
        return navigationCommit(
          () => publish.isCurrent() && modTabs.some((t) => t.id === current.id && t.path === oldPath),
          () => {
            if (!publish()) return false;
            current.path = path;
            current.label = name;
            return true;
          },
        );
      }
    }
    const existing = modTabs.find((t) => t.path && sameDocumentPath(t.path, path));
    if (existing) return prepareModTabActivation(existing.id);
    const publish = prepareViewChange("files", mainView.value === "files");
    if (!publish) return null;
    return navigationCommit(publish.isCurrent, () => {
      if (!publish()) return false;
      const id = `dir-${++modTabSeq}`;
      modTabs.push({ id, view: "files", icon: "📁", label: name, path });
      activeModTab.value = id;
      return true;
    });
  }
  function openDirTab(path: string, reuseCurrent = false): boolean {
    const publish = prepareDirTab(path, reuseCurrent);
    return publish ? publish() : false;
  }

  function prepareModTabActivation(id: string): ViewChangeCommit | null {
    const t = modTabs.find((x) => x.id === id);
    if (!t) return null;
    const p = t.path;
    const publish = prepareViewChange(t.view, id !== activeModTab.value);
    if (!publish) return null;
    return navigationCommit(
      () => publish.isCurrent() && modTabs.some((tab) => tab.id === id && tab.path === p),
      () => {
        if (!publish()) return false;
        activeModTab.value = id;
        // Keep the deferred public-boundary import: Browser already depends on Layout.
        // A late import must not overwrite another tab's URL or directory address.
        if (p) {
          import("../capabilities/browser/public").then(({ useBrowserStore }) => {
            if (activeModTab.value === id && mainView.value === t.view
                && modTabs.some((tab) => tab.id === id && tab.path === p)) {
              useBrowserStore().url = p;
            }
          });
        }
        return true;
      },
    );
  }
  function activateModTab(id: string): boolean {
    const publish = prepareModTabActivation(id);
    return publish ? publish() : false;
  }

  // Workspace calls this only after its native move/rename succeeds.
  // Updating the existing logical Owner also invalidates old async navigation.
  function relocateDirectoryTabs(from: string, to: string): void {
    const updates = modTabs.filter((tab) => !!tab.path).map((tab) => {
      const path = replaceDocumentPathPrefix(tab.path!, from, to);
      return { tab, path, label: documentPathName(path) };
    }).filter(({ tab, path }) => path !== tab.path);
    if (!updates.length) return;
    navigationRevision += 1;
    for (const { tab, path, label } of updates) {
      tab.path = path;
      tab.label = label;
    }
    const current = modTabs.find((tab) => tab.id === activeModTab.value && tab.view === mainView.value && tab.path);
    if (current?.path) {
      const id = current.id, path = current.path;
      import("../capabilities/browser/public").then(({ useBrowserStore }) => {
        if (activeModTab.value === id && mainView.value === current.view
            && modTabs.some((tab) => tab.id === id && tab.path === path)) useBrowserStore().url = path;
      });
    }
  }

  function prepareModTabClose(id: string): { publish: ViewChangeCommit; next?: ModTab; wasVisible: boolean } | null {
    const i = modTabs.findIndex((tab) => tab.id === id);
    if (i < 0) return null;
    const target = { ...modTabs[i] };
    const selected = activeModTab.value;
    const wasVisible = selected === id && mainView.value === target.view;
    const adjacent = modTabs[i + 1] || modTabs[i - 1];
    const next = adjacent ? { ...adjacent } : undefined;
    const publishView = wasVisible ? prepareViewChange(next?.view ?? "home", true) : null;
    if (wasVisible && !publishView) return null;
    const revision = navigationRevision;
    const publish = navigationCommit(
      () => revision === navigationRevision
        && modTabs.some((tab) => tab.id === id && tab.path === target.path)
        && (!next || modTabs.some((tab) => tab.id === next.id && tab.path === next.path)),
      () => {
        if (publishView && !publishView()) return false;
        const index = modTabs.findIndex((tab) => tab.id === id);
        modTabs.splice(index, 1);
        if (selected === id) activeModTab.value = next?.id ?? "";
        if (!publishView) navigationRevision += 1;
        return true;
      },
    );
    return { publish, next, wasVisible };
  }
  function closeModTab(id: string): boolean {
    const prepared = prepareModTabClose(id);
    return prepared ? prepared.publish() : false;
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
    exitSaveNotice,
    setExitSaveNotice,
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
    prepareViewChange,
    captureNavigation,
    prepareModTabActivation,
    prepareModTabClose,
    prepareDirTab,
    prepareModule,
    registerBeforeViewChange,
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
    relocateDirectoryTabs,
    activateModTab,
    closeModTab,
  };
});
