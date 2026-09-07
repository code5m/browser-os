import { defineStore } from "pinia";
import { reactive, computed, ref } from "vue";
import { bridge } from "../bridge";
import { useBrowserStore } from "./useBrowserStore";
import { useLayoutStore } from "./useLayoutStore";
import { useWorkspaceStore } from "./useWorkspaceStore";
import {
  HOME_APP_SESSION_ONLY_NOTICE,
  HOME_MAX_RECENTS,
  HOME_MAX_SHORTCUTS,
  HOME_PRINCIPAL_AREAS,
  HOME_SHORTCUT_LIMIT_MESSAGE,
  defaultShortcuts,
  homeAccessibleLabel,
  homeDisplayTarget,
  isStorageSafe,
  normalizeRecents,
  normalizeShortcuts,
  panelStateHome,
  pushRecent,
  recentFromShortcut,
  sanitizeShortcut,
  stableId,
  toPersisted,
  type HomeArea,
  type HomeAreaView,
  type HomeRecent,
  type HomeShortcut,
} from "../utils/homeUi";

// M5-W17 A3：类型与默认值的**单一真源**迁到 `src/utils/homeUi.ts`（纯逻辑层，可 headless 直测）。
// 此处原样再导出，保证既有消费方（HomePanel.vue / A5）的 import 路径与字段语义不变。
export type { HomeShortcut, HomeRecent, HomeArea, HomeAreaView } from "../utils/homeUi";

const STORAGE_KEY = "browser-os-home-shortcuts";
const RECENTS_KEY = "browser-os-home-recents-v1";
const DIRS_SEEDED_KEY = "browser-os-home-dirs-seeded-v2";

export const useHomeStore = defineStore("home", () => {
  const browser = useBrowserStore();
  const layout = useLayoutStore();
  const workspace = useWorkspaceStore();

  const shortcuts = reactive<HomeShortcut[]>(load());
  // M5-W17 A3：最近访问（有界 + 载入即校验，脏数据绝不进 UI）。
  const recents = reactive<HomeRecent[]>(loadRecents());
  const loading = ref(false);
  const error = ref<string | null>(null);
  const editing = reactive<{
    open: boolean;
    id: string; // 空串 = 新增
    type: "url" | "app" | "dir";
    name: string;
    target: string;
    icon: string;
  }>({ open: false, id: "", type: "url", name: "", target: "", icon: "🔗" });

  // 常用目录播种（仅首次）：把后端承诺的起始目录补进主页，旧版只播了前 4 个系统目录。
  async function seedDirShortcuts() {
    try {
      if (localStorage.getItem(DIRS_SEEDED_KEY)) return;
      const dirs = await bridge.getStartDirs();
      for (const d of dirs) {
        if (shortcuts.some((s) => s.target === d.path)) continue;
        const m = d.name.match(/^(\S+)\s(.+)$/); // "📁 桌面" → icon=📁 name=桌面
        addShortcut({
          type: "dir",
          name: m ? m[2] : d.name,
          target: d.path,
          icon: m ? m[1] : "📁",
          silent: true,
        });
      }
      save();
      localStorage.setItem(DIRS_SEEDED_KEY, "1");
    } catch {}
  }

  // M5-W17 A3：载入必须**逐字段校验**——旧实现直接返回 `JSON.parse` 的原始数组，
  // 缺 id / type 非法 / name 是 number 等脏数据会原样进 UI（渲染崩或显示 undefined）。
  // 校验后为空（数据损坏或用户已清空）→ 回落到稳定默认，保证首页永远可用。
  function load(): HomeShortcut[] {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const all = normalizeShortcuts(JSON.parse(raw));
        // 迁移期清理：旧数据里的 app 条目（命令体）不进内存，并随即从浏览器存储中抹除。
        const safe = all.filter(isStorageSafe);
        if (safe.length !== all.length) {
          try {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(safe));
          } catch {}
        }
        if (safe.length) return safe;
      }
    } catch {}
    return defaultShortcuts();
  }
  function loadRecents(): HomeRecent[] {
    try {
      const raw = localStorage.getItem(RECENTS_KEY);
      // 最近访问同理：app 条目的 target 也是命令体，不还原、不落库。
      if (raw) return normalizeRecents(JSON.parse(raw)).filter(isStorageSafe);
    } catch {}
    return [];
  }
  function save() {
    try {
      // HOME_NO_SECRET_PERSIST：只落库非敏感主页元数据，app 命令体**不写**浏览器存储。
      localStorage.setItem(STORAGE_KEY, JSON.stringify(toPersisted(shortcuts)));
    } catch {}
  }
  function saveRecents() {
    try {
      localStorage.setItem(RECENTS_KEY, JSON.stringify(toPersisted(recents)));
    } catch {}
  }

  const has = computed(() => shortcuts.length > 0);
  const hasRecents = computed(() => recents.length > 0);
  // 面板三态（共享验收 #2）：载入 / 空 / 就绪 / 错误，文案由纯逻辑层单一真源给出。
  const panelState = computed(() =>
    panelStateHome({ loading: loading.value, count: shortcuts.length, error: error.value }),
  );
  // 主要工作区入口（共享验收 #2/#3）：只读常量，**不新增路由、不新增命令**。
  const principalAreas = HOME_PRINCIPAL_AREAS;

  function openArea(view: HomeAreaView) {
    layout.setView(view);
  }
  // 安全展示（共享验收 #4）：目录/应用只给末段名，URL 只给 host+path，绝不回显完整路径或凭据。
  function displayTarget(s: HomeShortcut | HomeRecent) {
    return homeDisplayTarget(s);
  }
  function accessibleLabel(s: HomeShortcut | HomeRecent) {
    return homeAccessibleLabel(s);
  }
  function removeRecent(id: string) {
    const i = recents.findIndex((r) => r.id === id);
    if (i >= 0) {
      recents.splice(i, 1);
      saveRecents();
    }
  }
  function clearRecents() {
    recents.splice(0, recents.length);
    saveRecents();
  }

  // M5-W17 A3：id 改为**确定性派生**（同 type+target ⇒ 同 id）。随机 id 会导致
  // 每次载入渲染 key 漂移，且脏数据里的重复 id 无法去重。
  function makeId(type: HomeShortcut["type"], target: string) {
    return stableId(type, target);
  }

  function fileNameFromPath(path: string) {
    return path.replace(/\/+$/, "").split("/").filter(Boolean).pop() || path || "/";
  }

  function looksLikeDir(path: string) {
    return path.trim().startsWith("/") || path.trim().startsWith("~") || /^[A-Za-z]:[\\/]/.test(path.trim());
  }

  function addShortcut(input: {
    type: HomeShortcut["type"];
    name: string;
    target: string;
    icon?: string;
    silent?: boolean;
  }) {
    // M5-W17 A3：先过纯逻辑层校验（截断超长、丢弃非法 type / 空 name|target），再落库与渲染。
    const clean = sanitizeShortcut({
      id: "",
      type: input.type,
      name: input.name,
      target: input.target,
      icon: input.icon || "🔗",
    });
    if (!clean) return false;
    const existing = shortcuts.find((s) => s.type === clean.type && s.target === clean.target);
    if (existing) {
      existing.name = clean.name;
      existing.icon = clean.icon;
    } else {
      // 有界：新增前判上限（不再静默丢弃最旧，而是明确告知用户）。
      if (shortcuts.length >= HOME_MAX_SHORTCUTS) {
        if (!input.silent) layout.showToast(HOME_SHORTCUT_LIMIT_MESSAGE);
        return false;
      }
      shortcuts.push({ ...clean, id: makeId(clean.type, clean.target) });
    }
    save();
    if (!input.silent) {
      // app 条目不会落库，明确告知用户「仅本次会话有效」，避免重启后丢失造成的困惑。
      layout.showToast(
        clean.type === "app" ? HOME_APP_SESSION_ONLY_NOTICE : `已收藏到主页: ${clean.name}`,
      );
    }
    return true;
  }

  function favoriteCurrentPage() {
    const target = (browser.activeTab?.url || browser.url || "").trim();
    if (!target || looksLikeDir(target) || target === "about:blank") {
      layout.showToast("当前没有可收藏网页");
      return;
    }
    addShortcut({
      type: "url",
      name: browser.activeTab?.title || target,
      target,
      icon: "⭐",
    });
  }

  function favoriteCurrentDir() {
    const target = (layout.mainView === "files" ? workspace.filePath : browser.url).trim();
    if (!target || !looksLikeDir(target)) {
      layout.showToast("当前没有可收藏目录");
      return;
    }
    addShortcut({
      type: "dir",
      name: fileNameFromPath(target),
      target,
      icon: "📁",
    });
  }

  // 打开快捷方式：url → 内嵌浏览器；app → 启动系统应用；dir → 文件视图（IDE 树）
  async function open(s: HomeShortcut) {
    if (s.type === "url") {
      browser.url = s.target;
      layout.setView("browser");
      await browser.openBrowser();
    } else if (s.type === "dir") {
      // M0-4.b：改为静态引入。useWorkspaceStore 已被 App.vue 等十余处静态引入，
      // 这里的动态 import 既不会分包（Vite 会报 mix 告警），也不构成循环依赖。
      const ws = useWorkspaceStore();
      browser.url = s.target; // 地址栏同步显示目录路径
      layout.openDirTab(s.target);
      await ws.enterDir(s.target); // enterDir 内部会同步文件树根
      layout.showToast("📁 " + s.name);
    } else {
      try {
        await bridge.launchApp(s.target);
        layout.showToast("已启动: " + s.name);
      } catch {
        // M5-W17 A3（共享验收 #4）：启动失败**不回显原始错误串**——其中可能含
        // 本地绝对路径 / URL 参数 / 凭据；只给确定且无敏感信息的提示。
        layout.showToast("启动失败，请检查该快捷方式的目标是否有效。");
        return;
      }
    }
    // 仅成功路径记录最近访问：有界 + 去重 + 持久化（失败不污染最近列表）。
    const next = pushRecent(recents.slice(), recentFromShortcut(s, Date.now()), HOME_MAX_RECENTS);
    recents.splice(0, recents.length, ...next);
    saveRecents();
  }

  // ===== 编辑 =====
  function startAdd() {
    editing.open = true;
    editing.id = "";
    editing.type = "url";
    editing.name = "";
    editing.target = "";
    editing.icon = "🔗";
  }
  function startEdit(s: HomeShortcut) {
    editing.open = true;
    editing.id = s.id;
    editing.type = s.type;
    editing.name = s.name;
    editing.target = s.target;
    editing.icon = s.icon;
  }
  function cancelEdit() {
    editing.open = false;
    editing.id = "";
  }
  function saveEdit() {
    // M5-W17 A3：编辑保存同样走校验（超长截断、非法 type 拒绝）。
    const clean = sanitizeShortcut({
      id: editing.id,
      type: editing.type,
      name: editing.name,
      target: editing.target,
      icon: editing.icon || "🔗",
    });
    if (!clean) {
      layout.showToast("请填写名称与目标");
      return;
    }
    if (editing.id) {
      const s = shortcuts.find((x) => x.id === editing.id);
      if (s) {
        s.type = clean.type;
        s.name = clean.name;
        s.target = clean.target;
        s.icon = clean.icon;
      }
    } else {
      if (shortcuts.length >= HOME_MAX_SHORTCUTS) {
        layout.showToast(HOME_SHORTCUT_LIMIT_MESSAGE);
        return;
      }
      shortcuts.push({ ...clean, id: makeId(clean.type, clean.target) });
    }
    save();
    editing.open = false;
    editing.id = "";
    layout.showToast(clean.type === "app" ? HOME_APP_SESSION_ONLY_NOTICE : "已保存快捷方式");
  }
  function remove(id: string) {
    const i = shortcuts.findIndex((x) => x.id === id);
    if (i >= 0) {
      shortcuts.splice(i, 1);
      save();
      layout.showToast("已删除");
    }
  }
  function resetDefault() {
    shortcuts.splice(0, shortcuts.length, ...defaultShortcuts());
    save();
    layout.showToast("已恢复默认");
  }

  return {
    // 既有契约（保持字段名与语义不变，HomePanel.vue / A5 可直接消费）
    shortcuts,
    editing,
    has,
    open,
    seedDirShortcuts,
    favoriteCurrentPage,
    favoriteCurrentDir,
    startAdd,
    startEdit,
    cancelEdit,
    saveEdit,
    remove,
    resetDefault,
    // M5-W17 A3 新增：有界最近访问 + 主要工作区入口 + 安全展示/无障碍 + 面板三态
    recents,
    hasRecents,
    loading,
    error,
    panelState,
    principalAreas,
    openArea,
    displayTarget,
    accessibleLabel,
    removeRecent,
    clearRecents,
  };
});
