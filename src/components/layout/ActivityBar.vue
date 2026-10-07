<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import {
  useLayoutStore,
  TOP_NAV_ITEMS,
  NAV_MENU_SECTIONS,
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
import { Search, PanelLeftClose, PanelLeftOpen } from "@lucide/vue";
import { useWorkbenchStore } from "../../stores/useWorkbenchStore";
import { contributionRegistry } from "../../capability/contribution/registry";
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types";
const workbench = useWorkbenchStore();

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
const trailingActions = computed(() => contributionRegistry.getNavigationContributions(
  CONTRIBUTION_SLOTS.ACTIVITY_BAR_TRAILING,
));
const activityNavContributions = computed(() => contributionRegistry.getSurfaceContributions(
  CONTRIBUTION_SLOTS.ACTIVITY_BAR_NAV,
));
const activityRowContributions = computed(() => contributionRegistry.getSurfaceContributions(
  CONTRIBUTION_SLOTS.ACTIVITY_BAR_ROWS,
));

// 一级入口与 ☰ 菜单分节统一来自 useLayoutStore（W17 导航真源），
// 窄窗口按 navTopViews 从尾部裁剪，被裁掉的入口在 ☰ 菜单中仍可达。
const menuSections = NAV_MENU_SECTIONS;
const topItems = computed(() => TOP_NAV_ITEMS.filter((i) => layout.navTopViews.includes(i.view) && ['home','browser'].includes(i.view)));

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
const omniShowHistory = ref(true);
const omniShowCommon = ref(true);

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
  if (looksLikeDir(browser.url)) openDirCenter();
  else browser.openBrowser();
}
function pickDir(p: string) {
  layout.navSection = "";
  browser.url = p;
  openDirCenter();
}
function pickUrl(u: string) {
  layout.navSection = "";
  browser.url = u;
  browser.openBrowser();
}
// 目录在中央主区展示（与网页同一套展示区域）
async function openDirCenter() {
  const p = browser.url.trim();
  if (!p) {
    layout.showToast("请输入目录路径");
    return;
  }
  layout.browserDockOpen = false;
  layout.openDirTab(p);
  layout.leftTab = "files";
  await fs.enterDir(p);
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

      <!-- ☰ 菜单：展开工作区/工具/同步 -->
      <button
        data-nav-item
        data-nav-toggle="more"
        :class="{ active: layout.navSection === 'more' || menuSections.some((s) => s.items.some((c) => isNavActive(layout.mainView, c.view))) }"
        aria-label="更多功能"
        :aria-expanded="layout.navSection === 'more'"
        aria-controls="nav-more-row"
        title="更多功能"
        @click.stop="toggleSection('more')"
      >
        <span class="ic">☰</span>
        <span class="lab">菜单</span>
      </button>

      <button class="tbtn" title="统一命令" aria-label="统一命令" @click="workbench.commandOpen = !workbench.commandOpen"><Search :size="16" /></button>
      <button class="tbtn" :title="workbench.collapsed ? '恢复工具窗' : '折叠工具窗'" aria-label="折叠或恢复工具窗" @click="workbench.toggleTools()"><PanelLeftOpen v-if="workbench.collapsed" :size="16"/><PanelLeftClose v-else :size="16"/></button>
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
            placeholder="输入网址或目录路径（如 baidu.com 或 /home/you/Documents），回车前往"
            @keyup.enter="onAddrGo"
            @focus="layout.navSection = 'omni'"
          />
          <!-- M1-3：⭐ 收藏当前网页 + 📑 展开收藏夹侧栏（经通用 Contribution Registry 渲染） -->
          <template v-for="c in addressBarActions" :key="c.id">
            <component :is="c.component" />
          </template>
        </div>
        <button class="go" @click="onAddrGo">前往</button>
      </div>

      <!-- 右：浏览辅助 + 采集 + 设置 -->
      <!-- 收藏夹：任意视图均可打开；经通用 Contribution Registry 渲染（Shell 零 Bookmark 专属知识） -->
      <template v-for="c in trailingActions" :key="c.id">
        <component :is="c.component" />
      </template>
      <template v-if="layout.mainView === 'browser'">
        <button class="tbtn" aria-label="边浏览边管理文件" @click="layout.toggleBrowserDock('files')" title="边浏览边管理文件">🗂</button>
        <button class="tbtn" aria-label="边浏览边开终端" @click="layout.toggleBrowserDock('term')" title="边浏览边开终端">💻</button>
        <button class="tbtn" aria-label="精简模式" @click="layout.toggleCompact" title="精简模式：隐藏工具栏给网页更大空间">⛶</button>
      </template>
      <span class="sep"></span>
      <button
        class="sys"
        data-nav-item
        :class="{ active: browser.aiNavOpen }"
        aria-label="AI 导航"
        title="AI 导航"
        @click="browser.toggleAiNav()"
      >
        <span class="ic">🤖</span>
      </button>
      <button class="collect" data-nav-item aria-label="采集选中内容" :title="'采集选中内容'" @click="art.collectSelection">
        <span class="ic">📥</span>
        <span class="lab">采集</span>
      </button>
      <button class="sys" data-nav-item aria-label="系统设置" title="系统设置" @click="onItem('settings')">
        <span class="ic">⚙️</span>
      </button>
    </nav>

    <template v-for="c in activityRowContributions" :key="c.id">
      <component :is="c.component" />
    </template>

    <!-- 功能菜单扩展行 -->
    <div v-if="layout.navSection === 'more'" id="nav-more-row" class="expand-row">
      <template v-for="s in menuSections" :key="s.title">
        <span class="er-label">{{ s.title }}</span>
        <button
          v-for="c in s.items"
          :key="c.view"
          :class="{ active: isNavActive(layout.mainView, c.view) }"
          :aria-label="c.label"
          :aria-current="isNavActive(layout.mainView, c.view) ? 'page' : undefined"
          @click="onItem(c.view)"
        >
          <span class="ic">{{ c.icon }}</span> {{ c.label }}
        </button>
        <span class="er-sep"></span>
      </template>
      <button class="er-close" aria-label="收起功能菜单" @click="layout.navSection = ''" title="收起">✕</button>
    </div>

    <!-- 最近网址 / 最近常用目录扩展行（历史/常用可独立开关） -->
    <div v-if="layout.navSection === 'omni'" id="nav-omni-row" class="expand-row omni-row">
      <button :class="{ active: omniShowHistory }" aria-label="显示或隐藏历史记录" title="显示/隐藏历史记录" @click="omniShowHistory = !omniShowHistory">🕒 历史</button>
      <button :class="{ active: omniShowCommon }" aria-label="显示或隐藏常用目录" title="显示/隐藏常用目录" @click="omniShowCommon = !omniShowCommon">📂 常用</button>
      <span class="er-sep"></span>
      <template v-if="omniShowHistory && ws.recents.some((r) => r.type === 'url')">
        <button
          v-for="r in ws.recents.filter((r) => r.type === 'url').slice(0, 8)"
          :key="'u' + r.path"
          class="chip"
          :title="safeLabel(r.path)"
          @click="pickUrl(r.path)"
        >🌐 {{ safeLabel(r.path) }}</button>
      </template>
      <template v-if="omniShowHistory && recentDirs.length">
        <button v-for="d in recentDirs" :key="'d' + d" class="chip" :title="safeLabel(d)" @click="pickDir(d)">📁 {{ safeLabel(d) }}</button>
      </template>
      <template v-if="omniShowCommon && fs.startDirs.length">
        <button v-for="d in fs.startDirs" :key="'s' + d.path" class="chip" :title="safeLabel(d.path)" @click="pickDir(d.path)">📂 {{ d.name }}</button>
      </template>
      <span
        v-if="(omniShowHistory && !ws.recents.length && !recentDirs.length) && (omniShowCommon && !fs.startDirs.length)"
        class="er-label"
      >
        暂无记录：输入网址或 / 开头的目录路径，回车即自动识别
      </span>
      <button class="er-close" aria-label="收起最近与常用" @click="layout.navSection = ''" title="收起">✕</button>
    </div>
  </div>
</template>

<style scoped>
.tbar {
  flex-shrink: 0;
}
.activity{height:30px;box-sizing:border-box;background:#f5f7f8;color:#314651;border-bottom:1px solid #d9e0e3;gap:3px;padding:2px 6px}
.activity button{color:#425b68;background:transparent;border-radius:4px}
.activity button.active,.activity button.go{background:#e0eee8;color:#135b48}
.activity .addr-mid{flex:1;min-width:0}.activity .omni-wrap input{background:#fff;border:1px solid #d5dfe3;height:24px;color:#293c47}
.activity .collect{background:none}.activity .lab{display:none}
.caret-btn {
  min-width: 16px;
  padding: 2px 3px 2px 0;
  font-size: 9px;
  color: #7d8794;
}
.caret-btn.active {
  color: #fff;
}
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
.expand-row button:hover {
  border-color: #2b6cb0;
  color: #2b6cb0;
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
.er-warn {
  font-size: 11px;
  color: #c0392b;
  user-select: none;
}
.er-sep {
  width: 1px;
  height: 16px;
  background: #d5dbe7;
}
.er-primary {
  background: #2b6cb0 !important;
  border-color: #2b6cb0 !important;
  color: #fff !important;
}
.er-danger {
  color: #c33 !important;
}
.er-close {
  margin-left: auto;
  border: none !important;
  background: transparent !important;
  color: #999 !important;
}
.er-url {
  display: inline-flex;
  align-items: center;
  gap: 3px;
}
.er-gidx {
  font-size: 10px;
  color: #86909c;
}
.er-url input {
  width: 170px;
  height: 22px;
  border: 1px solid #d5dbe7;
  border-radius: 4px;
  font-size: 11px;
  padding: 0 6px;
  outline: none;
}
.omni-row {
  max-height: 74px;
  overflow-y: auto;
}
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
  outline: none;
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
  background: transparent;
  color: #cbd5e0;
  border-radius: 5px;
  cursor: pointer;
  font-size: 13px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 4px;
}
.tbtn:hover {
  background: #2f3a47;
  color: #fff;
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
.nav-icon .omni-wrap input {
  font-size: 11px;
}
</style>
