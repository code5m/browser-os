<script setup lang="ts">
import { useLayoutStore } from "../../stores/useLayoutStore";
import { useBrowserStore } from "../../capabilities/browser/public";
import { bridge } from "../../bridge";
import { ref, computed } from "vue";
import { useFileStore } from "../../capabilities/workspace/public";
import { useAppsStore } from "../../capabilities/apps/public";
import { useTerminalStore } from "../../capabilities/terminal/public";
import { getCurrentWindow } from "@tauri-apps/api/window";

// 统一页签条：浏览器网页页签 + 目录页签 + 模块页签（终端/文件等）混排在同一条，
// 不区分类型 —— 点击即切换对应视图，行为与浏览器标签一致。
const layout = useLayoutStore();
const browser = useBrowserStore();
const fs = useFileStore();
const appsStore = useAppsStore();
const term = useTerminalStore();
const context = ref<{ id: string; kind: 'web' | 'module' } | null>(null);

// Resolve the native window only after an explicit user action so a browser/dev
// preview can render the shell even when Tauri APIs are unavailable.
async function minimizeWindow() {
  try { await getCurrentWindow().minimize(); } catch { /* browser preview */ }
}

async function toggleWindowMaximize() {
  try { await getCurrentWindow().toggleMaximize(); } catch { /* browser preview */ }
}

async function closeWindow() {
  try { await getCurrentWindow().close(); } catch { /* browser preview */ }
}

async function startWindowDrag(event: MouseEvent) {
  if (event.button !== 0) return;
  const target = event.target as HTMLElement | null;
  if (target?.closest("button, .tab, input, select, textarea, a")) return;
  // One native request per press. Do not also register pointerdown or Tauri's
  // document-level data-tauri-drag-region handler on this titlebar.
  event.preventDefault();
  event.stopPropagation();
  try {
    if (event.detail === 2) await getCurrentWindow().toggleMaximize();
    else await getCurrentWindow().startDragging();
  } catch {
    layout.showToast("窗口拖动失败，请完全退出旧客户端后重新启动");
  }
}
async function closeContext(others = false) {
  const target = context.value;
  context.value = null;
  if (!target) return;
  if (target.kind === 'web') await browser.tabClose(target.id);
  else {
    const ids = others ? layout.modTabs.filter(t => t.id !== target.id).map(t => t.id) : [target.id];
    for (const id of ids) layout.closeModTab(id);
  }
}

// 取右键目标所处的页签列表（web 与 module 各用各的列表，互不串）
function listFor(kind: 'web' | 'module') {
  return kind === 'web' ? browser.tabs : layout.modTabs;
}

async function closeMany(kind: 'web' | 'module', ids: string[]) {
  for (const id of ids) {
    if (kind === 'web') await browser.tabClose(id);
    else layout.closeModTab(id);
  }
}

// 关闭当前页签左侧的所有页签
async function closeLeft() {
  const c = context.value;
  context.value = null;
  if (!c) return;
  const list = listFor(c.kind);
  const idx = list.findIndex((t) => t.id === c.id);
  if (idx <= 0) return;
  await closeMany(c.kind, list.slice(0, idx).map((t) => t.id));
}

// 关闭当前页签右侧的所有页签
async function closeRight() {
  const c = context.value;
  context.value = null;
  if (!c) return;
  const list = listFor(c.kind);
  const idx = list.findIndex((t) => t.id === c.id);
  if (idx < 0) return;
  await closeMany(c.kind, list.slice(idx + 1).map((t) => t.id));
}

// 关闭除当前页签外的所有页签
async function closeOthers() {
  const c = context.value;
  context.value = null;
  if (!c) return;
  const list = listFor(c.kind);
  await closeMany(c.kind, list.filter((t) => t.id !== c.id).map((t) => t.id));
}

// 当前右键页签若是本地目录/文件（module 类且带 path），返回其绝对路径，否则 null
const contextPath = computed(() => {
  const c = context.value;
  if (!c || c.kind !== 'module') return null;
  const t = layout.modTabs.find((x) => x.id === c.id);
  return t?.path ?? null;
});

// 相对路径：相对到命中的最长“起始目录”前缀；无匹配时退化为文件名
function relativeOf(abs: string): string {
  const dirs = (fs.startDirs || []).map((d) => d.path || "").filter(Boolean);
  let best = "";
  for (const d of dirs) {
    if (abs.startsWith(d) && d.length > best.length) best = d;
  }
  if (best) return abs.slice(best.length).replace(/^\/+/, "");
  const parts = abs.split("/");
  return parts[parts.length - 1] || abs;
}

async function revealHere() {
  const p = contextPath.value;
  context.value = null;
  if (!p) return;
  try {
    await bridge.revealPath(p);
  } catch (e: any) {
    layout.showToast("打开目录失败：" + (e?.message ?? e));
  }
}

async function copyAbsPath() {
  const p = contextPath.value;
  context.value = null;
  if (!p) return;
  await bridge.clipboardWrite(p);
  layout.showToast("已复制绝对路径");
}

async function copyRelPath() {
  const p = contextPath.value;
  context.value = null;
  if (!p) return;
  await bridge.clipboardWrite(relativeOf(p));
  layout.showToast("已复制相对路径");
}

async function activateWeb(id: string) {
  if (term.m0Cfg?.driver) return;
  // 本函数语义是"切到普通浏览器页签"：必须精确判断是否已处于 browser 视图。
  // 不能用 isBrowserView()（它把 grid 也算作 browser-like），否则宫格视图下
  // mainView 会停留在 grid、schedulePosition 走宫格分支，点击的页签不显示。
  if (layout.mainView !== "browser") layout.activateBrowser();
  await browser.tabSwitch(id);
}

function isActiveWeb(id: string) {
  return layout.mainView === "browser" && browser.activeTabId === id;
}

function isActiveMod(id: string) {
  return layout.isModView() && layout.activeModTab === id;
}

function activateMod(t: { id: string; view: string; path?: string }) {
  if (term.m0Cfg?.driver) return;
  layout.activateModTab(t.id);
  if (t.view === "apps") appsStore.loadApps();
  if (t.path) fs.enterDir(t.path);
  // Grid owns its lifecycle. Switching mainView to "grid" is sufficient:
  // useGridStore observes this intent and reuses/repositions the existing cells.
  // Browser store intentionally exposes no Grid lifecycle methods.
}
</script>

<template>
  <div class="tabbar unified" @mousedown="startWindowDrag">
    <!-- 网页页签 -->
    <div
      v-for="t in browser.tabs"
      :key="t.id"
      :class="['tab', { active: isActiveWeb(t.id) }]"
      @click="activateWeb(t.id)"
      @contextmenu.prevent="context = { id: t.id, kind: 'web' }"
      :title="t.url"
    >
      <span class="tab-ic">🌐</span>
      <span class="tab-title">{{ t.title || t.url }}</span>
      <button class="tab-close" @click.stop="browser.tabClose(t.id)" title="关闭">✕</button>
    </div>
    <!-- 目录 / 模块页签 -->
    <div
      v-for="t in layout.modTabs"
      :key="t.id"
      :class="['tab', { active: isActiveMod(t.id) }]"
      @click="activateMod(t)"
      @contextmenu.prevent="context = { id: t.id, kind: 'module' }"
      :title="t.path || t.label"
    >
      <span class="tab-ic">{{ t.icon }}</span>
      <span class="tab-title">{{ t.label }}</span>
      <button class="tab-close" @click.stop="layout.closeModTab(t.id)" title="关闭">✕</button>
    </div>
    <button class="tab-new" @click="browser.tabNew()" title="新建页签">＋</button>
    <div class="titlebar-drag" aria-hidden="true"></div>
    <div class="window-controls" aria-label="窗口控制">
      <button title="最小化" @click.stop="minimizeWindow">−</button>
      <button title="最大化或还原" @click.stop="toggleWindowMaximize">□</button>
      <button class="window-close" title="关闭窗口" @click.stop="closeWindow">×</button>
    </div>
  </div>
  <div v-if="context" class="tab-actions" role="menu" @keydown.esc="context = null">
    <button role="menuitem" @click="closeContext()">关闭</button>
    <button role="menuitem" @click="closeContext(true)">关闭其他</button>
    <template v-if="contextPath">
      <span class="tab-actions-sep" aria-hidden="true"></span>
      <button role="menuitem" @click="revealHere">在文件管理器中显示</button>
      <button role="menuitem" @click="copyAbsPath">复制绝对路径</button>
      <button role="menuitem" @click="copyRelPath">复制相对路径</button>
    </template>
    <button role="menuitem" @click="context = null">取消</button>
  </div>
</template>

<style scoped>
.tab-actions { display:flex; flex-wrap:wrap; align-items:center; flex:none; gap:6px; padding:4px 8px; background:#f2f4f6; border-bottom:1px solid #e5e6eb; }
.tab-actions button { border:1px solid #d5dbe7; background:#fff; color:#4e5969; font-size:12px; padding:3px 8px; border-radius:5px; cursor:pointer; }
.tab-actions button:hover { background:#eef1f6; color:#2b6cb0; }
.tab-actions button:disabled { color:#bbb; cursor:not-allowed; }
.tab-actions-sep { width:1px; align-self:stretch; background:#d5dbe7; margin:0 2px; }
.unified {
  display: flex;
  align-items: center;
  gap: 3px;
  padding: 3px 6px 0;
  background: #eef1f6;
  border-bottom: 1px solid #e5e6eb;
  flex-shrink: 0;
  overflow-x: auto;
  overflow-y: hidden;
  /* 页签超出时仍支持滚轮/触控板横向滚动，但不显示系统横向轨道。 */
  scrollbar-width: none;
  -ms-overflow-style: none;
  min-height: 32px;
}
.unified::-webkit-scrollbar { display: none; }
.titlebar-drag { flex: 1; min-width: 24px; align-self: stretch; cursor: move; }
.window-controls { display: flex; flex: none; align-items: stretch; margin: -3px -6px 0 0; }
.window-controls button { width: 34px; border: 0; border-radius: 0; background: transparent; display: grid; place-items: center; color: #667085; cursor: pointer; }
.window-controls button:hover { background: #dfe5ed; color: #1d2939; }
.window-controls .window-close:hover { background: #d64545; color: #fff; }
.tab {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  max-width: 200px;
  border: 1px solid #d5dbe7;
  border-bottom: none;
  background: #f7f8fa;
  color: #4e5969;
  border-radius: 7px 7px 0 0;
  padding: 4px 8px 4px 10px;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
}
.tab.active {
  background: #fff;
  color: #2b6cb0;
  font-weight: 600;
}
.tab-ic {
  font-size: 11px;
  flex-shrink: 0;
}
.tab-title {
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
.tab-close {
  border: none;
  background: transparent;
  color: #bbb;
  font-size: 10px;
  padding: 0 2px;
  border-radius: 3px;
  cursor: pointer;
  flex-shrink: 0;
}
.tab-close:hover {
  color: #c33;
  background: #f0f0f0;
}
.tab-new {
  border: none;
  background: transparent;
  color: #4e5969;
  font-size: 14px;
  padding: 2px 8px;
  border-radius: 5px;
  cursor: pointer;
  flex-shrink: 0;
}
.tab-new:hover {
  background: #e3e8f0;
}
</style>
