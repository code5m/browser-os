<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
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
// Reactive compatibility surface: downstream optional capabilities can add/remove legacy
// actions without leaving stale refs. Product v1 renders only one bookmark entry in address bar.
const trailingActions = computed(() => contributionRegistry.getNavigationContributions(
  CONTRIBUTION_SLOTS.ACTIVITY_BAR_TRAILING,
));
void trailingActions;
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
function syncWidth() {
  layout.setWindowWidth(window.innerWidth || 0);
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
function toggleSection(key: "" | "grid" | "more" | "omni") {
  layout.toggleNavSection(key);
}

// W17：活动条键盘漫游 —— ←/→ 在入口间环绕移动焦点，Home/End 直达首尾
function onNavKeydown(e: KeyboardEvent) {
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
// 最近访问行的分节开关：历史（最近网址+最近目录）/ 常用（常用目录）可独立控制
const omniIndex = ref(-1);
const omniCandidates = computed(() => {
  const seen = new Set<string>();
  const needle = browser.url.trim().toLowerCase();
  const candidates = [
    ...ws.recents.filter(r => r.type === "url").map(r => ({ value:r.path, label:safeLabel(r.path), type:"网页" })),
    ...recentDirs.value.map(value => ({ value, label:safeLabel(value), type:"最近目录" })),
    ...fs.startDirs.map(d => ({ value:d.path, label:safeLabel(d.path), type:"常用目录" })),
  ];
  return candidates.filter(c => { if (seen.has(c.value)) return false; seen.add(c.value); return !needle || c.value.toLowerCase().includes(needle); }).slice(0,6);
});
function chooseSuggestion(value:string) { if (looksLikeDir(value)) pickDir(value); else pickUrl(value); }
function onOmniKeydown(e:KeyboardEvent) {
  if (e.key === "ArrowDown" || e.key === "ArrowUp") { e.preventDefault(); omniIndex.value = (omniIndex.value + (e.key === "ArrowDown" ? 1 : -1) + omniCandidates.value.length) % (omniCandidates.value.length || 1); }
  else if (e.key === "Escape") { e.stopPropagation(); layout.closeNavSection(); omniIndex.value = -1; }
  else if (e.key === "Enter") { e.preventDefault(); const match = omniCandidates.value[omniIndex.value]; omniIndex.value = -1; if (match) chooseSuggestion(match.value); else onAddrGo(); }
}

// W17：最近网址/目录只做展示脱敏（点击仍用原始值），避免把带凭据的
// 查询串或 token 原样贴在活动条上。
function safeLabel(s: string): string {
  return redactSecrets(s || "");
}
function loadRecentDirs(): string[] {
  try {
    return JSON.parse(localStorage.getItem(RECENT_DIRS_KEY) || "[]");
  } catch {
    return [];
  }
}
function recordRecentDir(p: string) {
  const i = recentDirs.value.indexOf(p);
  if (i >= 0) recentDirs.value.splice(i, 1);
  recentDirs.value.unshift(p);
  recentDirs.value = recentDirs.value.slice(0, 12);
  try {
    localStorage.setItem(RECENT_DIRS_KEY, JSON.stringify(recentDirs.value));
  } catch {}
}
// 字段自动分辨：绝对路径 / ~ 开头 / Windows 盘符 → 目录；其余 → 网页
function looksLikeDir(s: string): boolean {
  const t = s.trim();
  return t.startsWith("/") || t.startsWith("~") || /^[A-Za-z]:[\\/]/.test(t);
}
function onAddrGo() {
  layout.navSection = "";
  if (looksLikeDir(browser.url)) void openDirCenter().catch(() => layout.showToast("无法打开目录，请检查路径"));
  else void browser.navigateCurrent().catch(() => layout.showToast("网页导航失败，请检查网址"));
}
function pickDir(p: string) {
  layout.navSection = "";
  browser.url = p;
  void openDirCenter().catch(() => layout.showToast("无法打开目录，请检查路径"));
}
function pickUrl(u: string) {
  layout.navSection = "";
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
        <div class="omni-wrap">
          <!-- 聚焦即自动展开最近/常用（无小三角）；选中或回车后自动收起 -->
          <input
            v-model="browser.url"
            aria-label="地址栏"
            data-nav-toggle="omni"
            :aria-expanded="layout.navSection === 'omni'"
            aria-controls="nav-omni-row"
            placeholder="输入网址或目录路径，回车打开"
            @keydown="onOmniKeydown"
            @focus="layout.navSection = 'omni'; omniIndex = -1"
          />
          <!-- M1-3：⭐ 收藏当前网页 + 📑 展开收藏夹侧栏（经通用 Contribution Registry 渲染） -->
          <template v-for="c in addressBarActions" :key="c.id">
            <component :is="c.component" />
          </template>
        </div>
 
      </div>

      <!-- 右：浏览辅助 + 采集 + 设置 -->
      <!-- 收藏夹：任意视图均可打开；经通用 Contribution Registry 渲染（Shell 零 Bookmark 专属知识） -->
      <template v-if="layout.mainView === 'browser'">
        <button class="tbtn" aria-label="边浏览边管理文件" @click="layout.toggleBrowserDock('files')" title="边浏览边管理文件">🗂</button>
        <button class="tbtn auxiliary-tool" aria-label="边浏览边开终端" @click="layout.toggleBrowserDock('term')" title="边浏览边开终端">💻</button>
        <button class="tbtn" aria-label="紧凑模式" @click="changeShellMode('compact')" title="紧凑模式：收起侧边工具栏">▤</button>
        <button class="tbtn" aria-label="沉浸模式" @click="toggleImmersive" title="沉浸模式：F11 可返回">⛶</button>
      </template>
      <span class="sep auxiliary-tool"></span>
      <button
        class="sys auxiliary-tool"
        data-nav-item
        :class="{ active: browser.aiNavOpen }"
        aria-label="AI 导航"
        title="AI 导航"
        @click="browser.toggleAiNav()"
      >
        <span class="ic">🤖</span>
      </button>
      <button class="collect auxiliary-tool" data-nav-item aria-label="采集选中内容" :title="'采集选中内容'" @click="art.collectSelection">
        <span class="ic">📥</span>
        <span class="lab">采集</span>
      </button>
      <button class="sys auxiliary-tool" data-nav-item aria-label="系统设置" title="系统设置" @click="onItem('settings')">
        <span class="ic">⚙️</span>
      </button>
    </nav>

    <template v-for="c in activityRowContributions" :key="c.id">
      <component :is="c.component" />
    </template>

    <!-- Native-safe suggestion surface: inline below the address bar, never above GTK WebView. -->
    <div v-if="layout.navSection === 'omni'" id="nav-omni-row" class="suggestion-list" role="listbox" aria-label="地址建议">
      <button v-for="(suggestion, i) in omniCandidates" :key="suggestion.value" role="option" :aria-selected="omniIndex === i"
        class="suggestion-item" :class="{ selected: omniIndex === i }" @mousedown.prevent @click="chooseSuggestion(suggestion.value)">
        <span class="suggestion-type">{{ suggestion.type === '网页' ? '🌐' : '📁' }}</span>
        <span class="suggestion-main">{{ suggestion.label }}</span><span class="suggestion-sub">{{ suggestion.type }}</span>
      </button>
      <p v-if="!omniCandidates.length" class="suggestion-empty">按 Enter 打开网址或本地目录</p>
    </div>
  </div>
</template>

<style scoped>
.tbar {
  flex-shrink: 0;
}
.activity{height:30px;box-sizing:border-box;background:#eef1f6;color:#314651;border-bottom:1px solid #d9e0e3;gap:3px;padding:2px 6px}
.activity button{color:#425b68;border-radius:4px}
.activity button.active,.activity button.go{background:#e0eee8;color:#135b48}
.activity .addr-mid{flex:1;min-width:0}

/* 扩展行：浅色、横排、按钮紧凑 */
.expand-row {
  display: flex;
  align-items: center;
  gap: 5px;
  flex-wrap: wrap;
  padding: 4px 8px;
  background: #f4f6fb;
  border-bottom: 1px solid #e3e7f5;
}
.expand-row button {
  border: 1px solid #d5dbe7;
  background: #fff;
  color: #4e5969;
  border-radius: 5px;
  padding: 3px 9px;
  font-size: 11px;
  cursor: pointer;
  white-space: nowrap;
}

.expand-row button.active {
  background: #2b6cb0;
  border-color: #2b6cb0;
  color: #fff;
}
.er-label {
  font-size: 11px;
  color: #86909c;
  user-select: none;
}
.er-sep {
  width: 1px;
  height: 16px;
  background: #d5dbe7;
}
.er-close {
  margin-left: auto;
  border: none !important;
  background: transparent !important;
  color: #999 !important;
}
.suggestion-list { display:flex; flex-direction:column; max-height:220px; overflow-y:auto; background:#fff; border:1px solid #dce3eb; border-top:0; box-shadow:0 8px 20px #13233b12; border-radius:0 0 12px 12px; padding:5px; }
.suggestion-item { display:flex; align-items:center; gap:12px; width:100%; padding:8px 12px; border:0; background:transparent; text-align:left; color:#243449; cursor:pointer; border-radius:7px; }
.suggestion-item:hover,.suggestion-item.selected { background:#eef4fb; }
.suggestion-type { width:20px; text-align:center; }
.suggestion-main { flex:1; min-width:0; overflow:hidden; white-space:nowrap; text-overflow:ellipsis; font-size:12px; }
.suggestion-sub { flex:none; font-size:11px; color:#667085; }
.suggestion-empty { padding:5px 12px; color:#667085; font-size:12px; margin:0; }
.chip {
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
}
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
.omni-wrap input:focus {
  border-color: #2b6cb0;
  background: #fff;
}
.addr-mid .go {
  background: #2b6cb0;
  color: #fff;
  border: none;
  border-radius: 5px;
  padding: 4px 12px;
  cursor: pointer;
  font-size: 12px;
  white-space: nowrap;
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
.nav-icon .addr-mid .go {
  display: none;
}

</style>

<style scoped>
/* 宽度不足时低频快捷入口退出第一层，保留搜索、双核心和工具中心。 */
@media (max-width: 950px) { .auxiliary-tool { display: none !important; } }
.activity :focus-visible { outline: 2px solid #3983c9; outline-offset: -2px; }
</style>
