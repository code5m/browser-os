<script setup lang="ts">
import { ref } from "vue";
import { useLayoutStore } from "../../stores/useLayoutStore";
import { useBrowserStore } from "../../stores/useBrowserStore";
import { useSystemStore } from "../../stores/useSystemStore";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";

const layout = useLayoutStore();
const browser = useBrowserStore();
const system = useSystemStore();
const ws = useWorkspaceStore();

// 一级入口：高频视图直达（主页/浏览/终端/剪贴板/知识库）
const topItems = [
  { view: "home", icon: "🏠", label: "主页" },
  { view: "browser", icon: "📁", label: "浏览" },
  { view: "term", icon: "💻", label: "终端" },
  { view: "clip", icon: "📋", label: "剪贴板" },
  { view: "arts", icon: "📚", label: "知识库" },
] as const;

// ☰ 菜单扩展行的分节内容
const menuSections = [
  {
    title: "工作区",
    items: [
      { view: "files", icon: "📂", label: "文件" },
    ],
  },
  {
    title: "工具",
    items: [
      { view: "apps", icon: "🚀", label: "应用" },
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

// 宫格设置扩展行数据
const gridLayouts = [
  { key: "grid", label: "▦ 宫格", title: "自动宫格平铺" },
  { key: "quad", label: "⊞ 四分", title: "四分 2×2" },
  { key: "horizontal", label: "▭ 横向", title: "横向一排" },
] as const;
const gridCounts = [2, 3, 4, 6, 9];
const urlsOpen = ref(false);

// ===== 资源监控（D）：宫格设置行"资源"按钮，2s 轮询 =====
import type { ResourceStats } from "../../types";
import { bridge } from "../../bridge";
import { onBeforeUnmount } from "vue";
const resOpen = ref(false);
const resStats = ref<ResourceStats | null>(null);
let resTimer: number | null = null;
async function resRefresh() {
  try {
    resStats.value = await bridge.resourceStats();
  } catch {}
}
function toggleRes() {
  resOpen.value = !resOpen.value;
  if (resOpen.value) {
    resRefresh();
    resTimer = window.setInterval(resRefresh, 2000);
  } else if (resTimer) {
    clearInterval(resTimer);
    resTimer = null;
  }
}
onBeforeUnmount(() => {
  if (resTimer) clearInterval(resTimer);
});
function fmtMb(mb: number) {
  return mb >= 1024 ? (mb / 1024).toFixed(1) + "G" : Math.round(mb) + "M";
}

// 扩展行：grid=宫格设置 / more=功能菜单 / omni=最近+常用。
// 关键设计：面板【不悬浮】——页签/宫格是原生 GTK 子窗口，永远压在 HTML 之上，
// 悬浮下拉必然被网页盖住。内联扩展行把工具栏撑高、网页随 viewport 整体下移
// （RO 触发重定位），零遮挡、零闪烁，也不需要"隐藏-恢复"的 hack。
const expanded = ref<"" | "grid" | "more" | "omni">("");

function toggleSection(key: "" | "grid" | "more" | "omni") {
  expanded.value = expanded.value === key ? "" : key;
}

function setGridLayout(mode: (typeof gridLayouts)[number]["key"]) {
  // 格数变化必须重建宫格（后端 webview 数量要与 gridCount 一致，
  // 只重排会对不存在的 grid-N 发定位 → tab not found）
  const countChanged = mode === "quad" && browser.gridCount !== 4;
  browser.gridLayout = mode;
  if (mode === "quad") browser.gridCount = 4;
  if (!browser.gridOpen) return;
  if (countChanged) browser.buildGrid();
  else browser.layoutGrid();
}

function setGridCount(n: number) {
  if (browser.gridCount === n) return;
  browser.gridCount = n;
  // 四分模式固定 4 格：选 2/3/6/9 格时自动切回自适应宫格，避免"2 格内容按 2×2 摆只显示上半"
  if (n !== 4 && browser.gridLayout === "quad") {
    browser.gridLayout = "grid";
  }
  if (browser.gridOpen) browser.buildGrid();
}

async function onItem(v: string) {
  expanded.value = "";
  // 离开宫格视图时自动关闭宫格：gridOpen 悬挂为 true 会让浏览视图的定位
  // 走错分支（tab 不复位、宫格被拉回可视区），且在非浏览器视图空转重试
  if (v !== "grid" && browser.gridOpen) await browser.closeGridAll();
  if (v === "apps") system.loadApps();
  if (v === "grid") {
    // 默认 AI 模式：点宫格直接出底部统一输入框（在 buildGrid 前设置，
    // 让输入框先于宫格定位渲染，首次布局即按"已缩矮"的 viewport 计算）
    browser.gridMode = "ai";
    // 宫格也走模块页签（去重复用），同时重建宫格内容
    layout.openModule("grid");
    // 总是重建宫格：buildGrid 内部 createGrid 会先 close_grid 再重建（幂等），
    // 避免 gridOpen 标志与后端宫格 webview 实际状态脱节导致的"有工具条没宫格"。
    browser.buildGrid();
    return;
  }
  // 浏览器主视图不是模块页签，直接切视图即可
  if (v === "browser") {
    layout.setView("browser");
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
  expanded.value = "";
  if (looksLikeDir(browser.url)) openDirCenter();
  else browser.openBrowser();
}
function pickDir(p: string) {
  expanded.value = "";
  browser.url = p;
  openDirCenter();
}
function pickUrl(u: string) {
  expanded.value = "";
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
  if (browser.gridOpen) await browser.closeGridAll();
  layout.browserDockOpen = false;
  layout.openDirTab(p);
  layout.leftTab = "files";
  await ws.enterDir(p);
  recordRecentDir(p);
  layout.showToast("📁 " + p);
}
</script>

<template>
  <div class="tbar">
    <nav class="activity">
      <!-- 左：视图导航 -->
      <button
        v-for="it in topItems"
        :key="it.view"
        :class="{ active: layout.mainView === it.view }"
        @click="onItem(it.view)"
        :title="it.label"
      >
        <span class="ic">{{ it.icon }}</span>
        <span class="lab">{{ it.label }}</span>
      </button>

      <!-- 宫格：左键直达打开，右侧 ▾ 展开设置行 -->
      <button
        :class="{ active: layout.mainView === 'grid' }"
        title="打开宫格"
        @click="onItem('grid')"
      >
        <span class="ic">🗂️</span>
        <span class="lab">宫格</span>
      </button>
      <button
        class="caret-btn"
        :class="{ active: expanded === 'grid' }"
        title="宫格设置"
        @click.stop="toggleSection('grid')"
      >▾</button>

      <!-- ☰ 菜单：展开工作区/工具/同步 -->
      <button
        :class="{ active: expanded === 'more' || menuSections.some((s) => s.items.some((c) => c.view === layout.mainView)) }"
        title="更多功能"
        @click.stop="toggleSection('more')"
      >
        <span class="ic">☰</span>
        <span class="lab">菜单</span>
      </button>

      <!-- 中：智能地址栏 -->
      <div class="addr-mid">
        <template v-if="layout.mainView === 'browser' || layout.mainView === 'grid'">
          <button class="tbtn" @click="browser.goBack" title="后退">←</button>
          <button class="tbtn" @click="browser.goForward" title="前进">→</button>
          <button class="tbtn" @click="browser.reloadActive" title="刷新">⟳</button>
        </template>
        <div class="omni-wrap">
          <!-- 聚焦即自动展开最近/常用（无小三角）；选中或回车后自动收起 -->
          <input
            v-model="browser.url"
            placeholder="输入网址或目录路径（如 baidu.com 或 /home/you/Documents），回车前往"
            @keyup.enter="onAddrGo"
            @focus="expanded = 'omni'"
          />
        </div>
        <button class="go" @click="onAddrGo">前往</button>
      </div>

      <!-- 右：浏览辅助 + 采集 + 设置 -->
      <template v-if="layout.mainView === 'browser'">
        <button class="tbtn" @click="layout.toggleBrowserDock('files')" title="边浏览边管理文件">🗂</button>
        <button class="tbtn" @click="layout.toggleBrowserDock('term')" title="边浏览边开终端">💻</button>
        <button class="tbtn" @click="layout.toggleCompact" title="精简模式：隐藏工具栏给网页更大空间">⛶</button>
      </template>
      <span class="sep"></span>
      <button class="collect" :title="'采集选中内容'" @click="ws.collectSelection">
        <span class="ic">📥</span>
        <span class="lab">采集</span>
      </button>
      <button class="sys" title="系统设置" @click="onItem('settings')">
        <span class="ic">⚙️</span>
      </button>
    </nav>

    <!-- AI 群发输入行：宫格打开且 AI 模式时常驻。
         走"内联扩展行撑高工具栏"的可靠模式（与宫格设置行同机制），
         宫格原生窗口随 viewport 下移，从机制上零遮挡——
         不要做成 viewport 底部栏，会被宫格 webview 盖住 -->
    <div v-if="browser.gridOpen && browser.gridMode === 'ai'" class="expand-row ai-send-row">
      <span class="er-label">🤖 群发</span>
      <input
        v-model="browser.gridAiInput"
        class="ai-send-input"
        placeholder="输入问题，同时发送给所有宫格中的 AI..."
        @keyup.enter="browser.gridSendAi"
      />
      <button class="er-primary" @click="browser.gridSendAi">发送</button>
    </div>

    <!-- 宫格设置扩展行 -->
    <div v-if="expanded === 'grid'" class="expand-row">
      <span class="er-label">模式</span>
      <button :class="{ active: browser.gridMode === 'browse' }" @click="browser.gridMode = 'browse'">🌐 浏览</button>
      <button :class="{ active: browser.gridMode === 'ai' }" @click="browser.gridMode = 'ai'">🤖 AI</button>
      <span class="er-sep"></span>
      <span class="er-label">布局</span>
      <button
        v-for="l in gridLayouts"
        :key="l.key"
        :class="{ active: browser.gridLayout === l.key }"
        :title="l.title"
        @click="setGridLayout(l.key)"
      >{{ l.label }}</button>
      <span class="er-sep"></span>
      <span class="er-label">格数</span>
      <button
        v-for="n in gridCounts"
        :key="n"
        :class="{ active: browser.gridCount === n }"
        @click="setGridCount(n)"
      >{{ n }}</button>
      <span class="er-sep"></span>
      <button class="er-primary" @click="browser.gridOpen ? browser.layoutGrid() : onItem('grid')">
        {{ browser.gridOpen ? "重排" : "打开" }}
      </button>
      <button :class="{ active: urlsOpen }" title="编辑各格网址" @click="urlsOpen = !urlsOpen">网址</button>
      <button :class="{ active: resOpen }" title="查看内存占用" @click="toggleRes">资源</button>
      <button class="er-danger" @click="expanded = ''; browser.closeGridAll()">关闭宫格</button>
      <button class="er-close" @click="expanded = ''" title="收起">✕</button>
    </div>
    <!-- 资源监控行：主进程 + 每宫格子进程树 RSS（2s 自动刷新） -->
    <div v-if="expanded === 'grid' && resOpen && resStats" class="expand-row">
      <span class="er-label">
        系统可用 {{ fmtMb(resStats.mem_available_mb) }} / {{ fmtMb(resStats.mem_total_mb) }}
      </span>
      <span class="er-sep"></span>
      <span class="er-label">应用共 {{ fmtMb(resStats.app_total_mb) }}</span>
      <span class="er-label">主进程 {{ fmtMb(resStats.main.rss_mb) }}</span>
      <span v-for="g in resStats.grids" :key="g.pid" class="er-label">
        {{ g.name }} {{ fmtMb(g.rss_mb) }}
      </span>
      <span v-if="resStats.mem_available_mb < 1500" class="er-warn">
        ⚠️ 可用内存偏低，建议减少格数或关闭其它应用
      </span>
    </div>
    <div v-if="expanded === 'grid' && urlsOpen" class="expand-row">
      <span v-for="i in browser.gridCount" :key="i" class="er-url">
        <span class="er-gidx">{{ i }}</span>
        <input v-model="browser.gridUrls[i - 1]" placeholder="网址" @keyup.enter="browser.gridSetUrl(i - 1)" />
        <button @click="browser.gridSetUrl(i - 1)">↺</button>
      </span>
    </div>

    <!-- 功能菜单扩展行 -->
    <div v-if="expanded === 'more'" class="expand-row">
      <template v-for="s in menuSections" :key="s.title">
        <span class="er-label">{{ s.title }}</span>
        <button
          v-for="c in s.items"
          :key="c.view"
          :class="{ active: layout.mainView === c.view }"
          @click="onItem(c.view)"
        >
          <span class="ic">{{ c.icon }}</span> {{ c.label }}
        </button>
        <span class="er-sep"></span>
      </template>
      <button class="er-close" @click="expanded = ''" title="收起">✕</button>
    </div>

    <!-- 最近网址 / 最近常用目录扩展行（历史/常用可独立开关） -->
    <div v-if="expanded === 'omni'" class="expand-row omni-row">
      <button :class="{ active: omniShowHistory }" title="显示/隐藏历史记录" @click="omniShowHistory = !omniShowHistory">🕒 历史</button>
      <button :class="{ active: omniShowCommon }" title="显示/隐藏常用目录" @click="omniShowCommon = !omniShowCommon">📂 常用</button>
      <span class="er-sep"></span>
      <template v-if="omniShowHistory && ws.recents.some((r) => r.type === 'url')">
        <button
          v-for="r in ws.recents.filter((r) => r.type === 'url').slice(0, 8)"
          :key="'u' + r.path"
          class="chip"
          :title="r.path"
          @click="pickUrl(r.path)"
        >🌐 {{ r.path }}</button>
      </template>
      <template v-if="omniShowHistory && recentDirs.length">
        <button v-for="d in recentDirs" :key="'d' + d" class="chip" :title="d" @click="pickDir(d)">📁 {{ d }}</button>
      </template>
      <template v-if="omniShowCommon && ws.startDirs.length">
        <button v-for="d in ws.startDirs" :key="'s' + d.path" class="chip" :title="d.path" @click="pickDir(d.path)">📂 {{ d.name }}</button>
      </template>
      <span
        v-if="(omniShowHistory && !ws.recents.length && !recentDirs.length) && (omniShowCommon && !ws.startDirs.length)"
        class="er-label"
      >
        暂无记录：输入网址或 / 开头的目录路径，回车即自动识别
      </span>
      <button class="er-close" @click="expanded = ''" title="收起">✕</button>
    </div>
  </div>
</template>

<style scoped>
.tbar {
  flex-shrink: 0;
}
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
</style>
