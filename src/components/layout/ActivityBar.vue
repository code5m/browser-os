<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  useLayoutStore,
  TOP_NAV_ITEMS,
  isNavActive,
  nextNavIndex,
} from "../../stores/useLayoutStore";
import { useBrowserStore } from "../../capabilities/browser/public";
import { useAppsStore } from "../../capabilities/apps/public";
import { useTerminalStore } from "../../capabilities/terminal/public";
import { useWorkspaceStore } from "../../capabilities/workspace/public";
import { useArtifactStore } from "../../capabilities/workspace/public";
import { useFileStore } from "../../capabilities/workspace/public";
import { redactSecrets } from "../../utils/redact";
import { filterOmnibox, isLocalPath, safeRecentDirs, type OmniboxItem } from "../../utils/omniboxSuggestions";
import { bridge } from "../../bridge";
import { contributionRegistry } from "../../capability/contribution/registry";
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types";

const layout = useLayoutStore();
const browser = useBrowserStore();
const appsStore = useAppsStore();
const term = useTerminalStore();
const ws = useWorkspaceStore();
const art = useArtifactStore();
const fs = useFileStore();

// 收藏夹等贡献：经通用 Contribution Registry 按 slot 遍历渲染。
// Shell 不持有 Bookmark 专属知识（不 import 其 store / ui），C3 关键（8B.1）。
const addressBarActions = computed(() => contributionRegistry.getNavigationContributions(
  CONTRIBUTION_SLOTS.ADDRESS_BAR_ACTIONS,
));
const omniContributions = computed(() => contributionRegistry.getSurfaceContributions(
  CONTRIBUTION_SLOTS.OMNIBOX_SUGGESTIONS,
));
const activityNavContributions = computed(() => contributionRegistry.getSurfaceContributions(
  CONTRIBUTION_SLOTS.ACTIVITY_BAR_NAV,
));
const activityRowContributions = computed(() => contributionRegistry.getSurfaceContributions(
  CONTRIBUTION_SLOTS.ACTIVITY_BAR_ROWS,
));

// 一级入口与 ☰ 菜单分节统一来自 useLayoutStore（W17 导航真源），
// 窄窗口按 navTopViews 从尾部裁剪，被裁掉的入口在 ☰ 菜单中仍可达。
const topItems = computed(() => TOP_NAV_ITEMS.filter((i) => layout.navTopViews.includes(i.view)));

// ===== W17：窗口宽度上报（窄窗口密度）+ resize 监听清理 =====
const navEl = ref<HTMLElement | null>(null);
const omniAnchor = ref<HTMLElement | null>(null);
// The native-safe suggestions row stays IN FLOW, but only spans the address
// field. A floating HTML overlay cannot cover the Linux WebKit child view.
const suggestionsGeometry = ref<Record<string, string>>({});
function syncSuggestionsGeometry() {
  const anchor = omniAnchor.value?.getBoundingClientRect();
  const nav = navEl.value?.getBoundingClientRect();
  if (!anchor || !nav || anchor.width <= 0) return;
  const left = Math.max(0, Math.round(anchor.left - nav.left));
  const width = Math.max(0, Math.min(Math.round(anchor.width), Math.floor(nav.width - left - 6)));
  suggestionsGeometry.value = { marginLeft: left + "px", width: width + "px" };
}
const moreActionsOpen = ref(false);
watch(() => layout.mainView, () => { moreActionsOpen.value = false; });
function toggleMoreActions() { layout.navSection = ""; moreActionsOpen.value = !moreActionsOpen.value; }
function syncWidth() {
  layout.setWindowWidth(window.innerWidth || 0);
  nextTick(syncSuggestionsGeometry);
}
onMounted(() => {
  syncWidth();
  window.addEventListener("resize", syncWidth);
});
onBeforeUnmount(() => {
  window.removeEventListener("resize", syncWidth);
});
// 扩展行：grid=宫格设置 / more=功能菜单 / omni=最近+常用。
// 关键设计：面板【不悬浮】——页签/宫格是原生 GTK 子窗口，永远压在 HTML 之上，
// 悬浮下拉必然被网页盖住。内联扩展行把工具栏撑高、网页随 viewport 整体下移
// （RO 触发重定位），零遮挡、零闪烁，也不需要"隐藏-恢复"的 hack。
// 扩展行状态提升到 store（layout.navSection）：视图切换/关闭都由 store 统一收起，
// 组件内不再自持一份，避免"换了视图扩展行还挂着"的状态分裂。
function changeShellMode(mode: "standard" | "compact" | "immersive") {
  window.dispatchEvent(new CustomEvent("browseros:set-shell-mode", { detail: mode }));
}
function toggleImmersive() {
  changeShellMode("immersive");
}

// W17：活动条键盘漫游 —— ←/→ 在入口间环绕移动焦点，Home/End 直达首尾
function onNavKeydown(e: KeyboardEvent) {
  const origin = e.target as HTMLElement;
  if (origin.tagName === "INPUT" || origin.tagName === "TEXTAREA" || origin.isContentEditable) return;
  const nav = navEl.value;
  if (!nav) return;
  const items = Array.from(nav.querySelectorAll<HTMLElement>("[data-nav-item]"));
  if (!items.length) return;
  const cur = items.indexOf(document.activeElement as HTMLElement);
  let next = -1;
  if (e.key === "ArrowRight") next = nextNavIndex(cur < 0 ? -1 : cur, 1, items.length);
  else if (e.key === "ArrowLeft") next = nextNavIndex(cur < 0 ? 0 : cur, -1, items.length);
  else if (e.key === "Home") next = 0;
  else if (e.key === "End") next = items.length - 1;
  else return;
  e.preventDefault();
  items[next]?.focus();
}

// W17：Esc 收起扩展行并把焦点还给触发它的按钮（键盘用户不会丢失焦点位置）
function onEscape() {
  if (moreActionsOpen.value) { moreActionsOpen.value = false; nextTick(() => document.querySelector<HTMLElement>("[data-nav-toggle=more-actions]")?.focus()); return; }
  const open = layout.navSection;
  if (!open) return;
  layout.closeNavSection();
  nextTick(() => {
    document.querySelector<HTMLElement>(`[data-nav-toggle="${open}"]`)?.focus();
  });
}

async function onItem(v: string) {
  if (term.m0Cfg?.driver) {
    bridge.debugLog(`[M0] ignore activity item ${v} while driver=${term.m0Cfg.driver}`);
    return;
  }
  layout.navSection = "";
  // 离开宫格视图**不再**自动关闭宫格（新语义 HIDE ≠ CLOSE）：资源存活但隐藏。
  // 历史曾因 gridOpen 悬挂为 true 导致浏览视图定位走错分支（tab 不复位、宫格被拉回
  // 可视区、非浏览器视图空转重试），现已由 syncViewVisibility 的 Visibility Controller
  // 按 desiredGridVisibility 统一收敛，不再依赖"离开即销毁"。
  if (v === "apps") appsStore.loadApps();
  // 浏览器主视图不是模块页签，直接切视图即可
  if (v === "browser") {
    layout.activateBrowser();
    return;
  }
  // 菜单功能与浏览器一致：点一个就新建/复用一个标签
  layout.openModule(v as any);
}

// ===== 智能地址栏（omnibox）：网址/目录自动识别，目录与网页都在中央主区展示 =====
const RECENT_DIRS_KEY = "browser-os-recent-dirs";
const recentDirs = ref<string[]>(loadRecentDirs());
const omniIndex = ref(-1);
// Only keystrokes trigger filtering: merely focusing an already-open URL must
// show all recent web/file/folder targets (and contributed bookmarks).
const omniTyped = ref(false);
const omniCandidates = computed(() => {
  const items: OmniboxItem[] = [
    ...ws.recents.filter(r => r.type === "url").map(r => ({ value: r.path, label: safeLabel(r.title || r.path), type: "最近网页", kind: "web" as const })),
    ...ws.recents.filter(r => r.type === "file").map(r => ({ value: r.path, label: safeLabel(r.title || r.path), type: "最近文件", kind: "file" as const })),
    ...recentDirs.value.map(value => ({ value, label: safeLabel(value), type: "最近目录", kind: "dir" as const })),
    ...fs.startDirs.map(d => ({ value: d.path, label: safeLabel(d.name || d.path), type: "常用目录", kind: "dir" as const })),
  ];
  return filterOmnibox(items, browser.url, omniTyped.value, layout.addrMode);
});
function safeLabel(s: string): string { return redactSecrets(s || ""); }
function loadRecentDirs(): string[] {
  try { return safeRecentDirs(JSON.parse(localStorage.getItem(RECENT_DIRS_KEY) || "[]")); }
  catch { return []; }
}
function recordRecentDir(path: string) {
  recentDirs.value = safeRecentDirs([path, ...recentDirs.value.filter(p => p !== path)]);
  try { localStorage.setItem(RECENT_DIRS_KEY, JSON.stringify(recentDirs.value)); } catch {}
}
function showOmnibox() {
  omniTyped.value = false;
  omniIndex.value = -1;
  layout.navSection = "omni";
  nextTick(syncSuggestionsGeometry);
}
function chooseSuggestion(s: OmniboxItem) {
  if (s.kind === "dir") pickDir(s.value);
  else if (s.kind === "file") {
    layout.closeNavSection();
    const record = ws.recents.find(r => r.type === "file" && r.path === s.value);
    if (record) ws.openRecent(record);
  } else pickUrl(s.value);
}
function onOmniKeydown(e: KeyboardEvent) {
  if (e.key === "ArrowDown" || e.key === "ArrowUp") {
    e.preventDefault();
    omniIndex.value = (omniIndex.value + (e.key === "ArrowDown" ? 1 : -1) + omniCandidates.value.length) % (omniCandidates.value.length || 1);
  } else if (e.key === "Escape") {
    e.stopPropagation(); layout.closeNavSection(); omniIndex.value = -1;
  } else if (e.key === "Enter") {
    e.preventDefault();
    const match = omniCandidates.value[omniIndex.value];
    omniIndex.value = -1;
    if (match) chooseSuggestion(match); else onAddrGo();
  }
}
function onAddrGo() {
  layout.closeNavSection();
  if (layout.addrMode === "dir" || (layout.addrMode === "auto" && isLocalPath(browser.url)))
    void openDirCenter().catch(() => layout.showToast("无法打开目录，请检查路径"));
  else void browser.navigateCurrent().catch(() => layout.showToast("网页导航失败，请检查网址"));
}
function pickDir(p: string) {
  layout.closeNavSection();
  browser.url = p;
  void openDirCenter().catch(() => layout.showToast("无法打开目录，请检查路径"));
}
function pickUrl(u: string) {
  layout.closeNavSection();
  browser.url = u;
  void browser.navigateCurrent().catch(() => layout.showToast("网页导航失败，请检查网址"));
}
// 目录在中央主区展示（与网页同一套展示区域）
async function openDirCenter() {
  const p = browser.url.trim();
  if (!p) {
    layout.showToast("请输入目录路径");
    return;
  }
  await fs.enterDir(p);
  await browser.consumeActiveBlankTab();
  layout.browserDockOpen = false;
  layout.openDirTab(p, true);
  layout.leftTab = "files";
  recordRecentDir(p);
  layout.showToast("📁 " + p);
}
</script>

<template>
  <div class="tbar" :class="'nav-' + layout.navDensity" @keydown.esc="onEscape">
    <nav ref="navEl" class="activity" aria-label="主导航" @keydown="onNavKeydown">
      <!-- 左：视图导航 -->
      <button
        v-for="it in topItems"
        :key="it.view"
        data-nav-item
        :class="{ active: isNavActive(layout.mainView, it.view) }"
        :aria-label="it.label"
        :aria-current="isNavActive(layout.mainView, it.view) ? 'page' : undefined"
        @click="onItem(it.view)"
        :title="it.label"
      >
        <span class="ic">{{ it.icon }}</span>
        <span class="lab">{{ it.label }}</span>
      </button>

      <template v-for="c in activityNavContributions" :key="c.id">
        <component :is="c.component" />
      </template>

      <!-- 中：智能地址栏 -->
      <div class="addr-mid">
        <template v-if="layout.mainView === 'browser' || layout.mainView === 'grid'">
          <button class="tbtn" aria-label="后退" @click="browser.goBack" title="后退">←</button>
          <button class="tbtn" aria-label="前进" @click="browser.goForward" title="前进">→</button>
          <button class="tbtn" aria-label="刷新" @click="browser.reloadActive" title="刷新">⟳</button>
        </template>
        <div ref="omniAnchor" class="omni-wrap">
          <!-- 聚焦即自动展开最近/常用（无小三角）；选中或回车后自动收起 -->
          <input
            v-model="browser.url"
            aria-label="地址栏"
            data-nav-toggle="omni"
            :aria-expanded="layout.navSection === 'omni'"
            :aria-activedescendant="omniIndex >= 0 ? 'omni-suggestion-' + omniIndex : undefined"
            aria-controls="nav-omni-row"
            placeholder="输入网址或目录路径，回车打开"
            @keydown="onOmniKeydown"
            @input="omniIndex = -1; omniTyped = true"
            @blur="layout.navSection = ''"
            @focus="showOmnibox"
          />
          <button type="button" class="omni-mode" aria-label="切换地址栏模式" :title="'地址栏模式：' + (layout.addrMode === 'auto' ? '自动' : layout.addrMode === 'url' ? '网页' : '文件')"
            @mousedown.prevent @click="layout.toggleAddrMode()">{{ layout.addrMode === 'auto' ? '自动 ▾' : layout.addrMode === 'url' ? '网页 ▾' : '文件 ▾' }}</button>
          <!-- Bookmark capability owns the favorite action; Shell only mounts contributions. -->
          <template v-for="c in addressBarActions" :key="c.id">
            <component :is="c.component" />
          </template>
        </div>

      </div>

      <!-- Keep only one contextual file shortcut and one overflow action on the default chrome. -->
      <button v-if="layout.mainView === 'browser'" class="tbtn" aria-label="文件侧栏" title="文件侧栏" @click="layout.toggleBrowserDock('files')">▣</button>
      <button class="tbtn action-overflow" data-nav-item data-nav-toggle="more-actions" aria-label="更多操作" title="更多操作"
        :aria-expanded="moreActionsOpen" aria-controls="chrome-more-actions" @click="toggleMoreActions">···</button>
    </nav>

    <!-- Native-safe suggestion surface: inline below the address bar, never above GTK WebView. -->
    <div v-if="layout.navSection === 'omni'" id="nav-omni-row" class="suggestion-list" :style="suggestionsGeometry" role="listbox" aria-label="地址建议">
      <button v-for="(suggestion, i) in omniCandidates" :key="suggestion.value" :id="'omni-suggestion-' + i" role="option" :aria-selected="omniIndex === i"
        class="suggestion-item" :class="{ selected: omniIndex === i }" @mousedown.prevent @click="chooseSuggestion(suggestion)">
        <span class="suggestion-type">{{ suggestion.kind === 'web' ? '🌐' : suggestion.kind === 'file' ? '📄' : '📁' }}</span>
        <span class="suggestion-main">{{ suggestion.label }}</span><span class="suggestion-sub">{{ suggestion.type }}</span>
      </button>
      <p v-if="!omniCandidates.length && !omniContributions.length" class="suggestion-empty">输入网址、文件或目录，按 Enter 打开</p>
      <component v-for="c in omniContributions" :key="c.id" :is="c.component"
        :query="omniTyped ? browser.url : ''" :mode="layout.addrMode"
        @choose-url="pickUrl" @choose-dir="pickDir" />
    </div>
    <div v-if="moreActionsOpen" id="chrome-more-actions" class="more-actions" role="group" aria-label="更多操作">
      <span class="more-heading">窗口</span>
      <button @click="changeShellMode(layout.shellMode === 'compact' ? 'standard' : 'compact');moreActionsOpen=false">{{ layout.shellMode === 'compact' ? '恢复标准模式' : '紧凑模式' }}</button>
      <button @click="toggleImmersive();moreActionsOpen=false">沉浸全屏 · F11</button>
      <template v-if="layout.mainView === 'browser'">
        <span class="more-divider"></span>
        <span class="more-heading">工作区</span>
        <button @click="layout.toggleBrowserDock('term');moreActionsOpen=false">终端侧栏</button>
        <button @click="browser.toggleAiNav();moreActionsOpen=false">AI 导航</button>
      </template>
      <span class="more-divider"></span>
      <span class="more-heading">管理</span>
      <button @click="art.collectSelection();moreActionsOpen=false">采集选中内容</button>
      <button @click="onItem('settings');moreActionsOpen=false">设置</button>
      <button @click="layout.navSection='';moreActionsOpen=false" aria-label="关闭更多操作">关闭</button>
    </div>
    <template v-for="c in activityRowContributions" :key="c.id">
      <component :is="c.component" />
    </template>


  </div>
</template>

<style scoped>
.tbar {
  flex-shrink: 0;
}
.activity{height:30px;box-sizing:border-box;background:#eef1f6;color:#314651;border-bottom:1px solid #d9e0e3;gap:3px;padding:2px 6px}
.activity button{color:#425b68;border-radius:4px}
.activity button.active{background:#e0eee8;color:#135b48}
.activity .addr-mid{flex:1;min-width:0}

.more-actions { display:flex; gap:6px; flex-wrap:wrap; align-items:center; padding:7px 12px; background:#f8fafc; border-bottom:1px solid #e2e8f0; }
.more-actions button { border:1px solid #d8e1eb; background:#fff; color:#344054; border-radius:7px; padding:5px 10px; font-size:12px; cursor:pointer; }
.more-actions button:hover { background:#eaf2fb; }
.more-heading { color:#667085; font-size:11px; font-weight:600; }
.more-divider { width:1px; height:18px; background:#dce4ee; margin:0 3px; }
.action-overflow { font-weight:700; font-size:17px; }
.activity button:hover { background:#e9eef6!important; color:#344054!important; }
.activity button.active { background:#e3eefb; color:#17548f; }
.suggestion-list { display:flex; flex-direction:column; box-sizing:border-box; max-height:220px; max-width:100%; overflow-y:auto; background:#fff; border:1px solid #dce3eb; border-top:0; box-shadow:0 8px 20px #13233b12; border-radius:0 0 12px 12px; padding:5px; }
.suggestion-item { display:flex; align-items:center; gap:12px; width:100%; padding:8px 12px; border:0; background:transparent; text-align:left; color:#243449; cursor:pointer; border-radius:7px; }
.suggestion-item:hover,.suggestion-item.selected { background:#eef4fb; }
.suggestion-type { width:20px; text-align:center; }
.suggestion-main { flex:1; min-width:0; overflow:hidden; white-space:nowrap; text-overflow:ellipsis; font-size:12px; }
.suggestion-sub { flex:none; font-size:11px; color:#667085; }
.suggestion-empty { padding:5px 12px; color:#667085; font-size:12px; margin:0; }
/* 中部智能地址栏 */
.addr-mid {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  margin: 0 6px;
}
.omni-wrap {
  position: relative;
  flex: 1;
  display: flex;
  min-width: 0;
}
.omni-wrap input {
  flex: 1;
  height: 24px;
  border: 1px solid #3a4452;
  border-radius: 12px;
  padding: 0 12px;
  font-size: 12px;
  min-width: 0;
  background: #f4f6f9;
  color: #333;
}
.omni-mode { flex:none; border:0; border-radius:7px; background:#eef4fb; color:#245c9b; padding:3px 6px; cursor:pointer; font-size:11px; }
.omni-wrap input:focus {
  border-color: #2b6cb0;
  background: #fff;
}
.tbtn {
  min-width: 24px;
  height: 24px;
  border: none;
  cursor: pointer;
  font-size: 13px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 4px;
}

/* W17 窄窗口密度：compact 先收文字标签，icon 再收次要按钮。
   裁剪掉的入口不会消失——它们仍在 ☰ 菜单里（由 store 的 navTopViews 保证）。 */
.nav-compact .lab,
.nav-icon .lab {
  display: none;
}
.nav-compact .activity button,
.nav-icon .activity button {
  padding: 2px 6px;
}

</style>

<style scoped>
.activity :focus-visible { outline: 2px solid #3983c9; outline-offset: -2px; }
</style>
