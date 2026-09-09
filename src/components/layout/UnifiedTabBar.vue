<script setup lang="ts">
import { useLayoutStore } from "../../stores/useLayoutStore";
import { useBrowserStore } from "../../stores/useBrowserStore";
import { ref } from "vue";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import { useSystemStore } from "../../stores/useSystemStore";
import { getCurrentWindow } from "@tauri-apps/api/window";

// 统一页签条：浏览器网页页签 + 目录页签 + 模块页签（终端/文件等）混排在同一条，
// 不区分类型 —— 点击即切换对应视图，行为与浏览器标签一致。
const layout = useLayoutStore();
const browser = useBrowserStore();
const ws = useWorkspaceStore();
const system = useSystemStore();
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
  try { await getCurrentWindow().startDragging(); } catch { /* browser preview */ }
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

async function activateWeb(id: string) {
  if (system.m0Cfg?.driver) return;
  if (!layout.isBrowserView()) layout.setView("browser");
  await browser.tabSwitch(id);
}

function isActiveWeb(id: string) {
  return layout.isBrowserView() && browser.activeTabId === id;
}

function isActiveMod(id: string) {
  return layout.isModView() && layout.activeModTab === id;
}

function activateMod(t: { id: string; view: string; path?: string }) {
  if (system.m0Cfg?.driver) return;
  layout.activateModTab(t.id);
  if (t.view === "apps") system.loadApps();
  if (t.path) ws.enterDir(t.path);
  // 宫格页签被关闭后重新激活时，必须重建宫格 webview 内容
  if (t.view === "grid") { if (browser.gridOpen) browser.layoutGrid(); else browser.buildGrid(); }
}
</script>

<template>
  <div class="tabbar unified">
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
    <div class="titlebar-drag" data-tauri-drag-region aria-hidden="true" @mousedown="startWindowDrag"></div>
    <div class="window-controls" aria-label="窗口控制">
      <button title="最小化" @click.stop="minimizeWindow">−</button>
      <button title="最大化或还原" @click.stop="toggleWindowMaximize">□</button>
      <button class="window-close" title="关闭窗口" @click.stop="closeWindow">×</button>
    </div>
  </div>
  <div v-if="context" class="tab-actions" role="menu" @keydown.esc="context = null">
    <button role="menuitem" @click="closeContext()">关闭页签</button>
    <button role="menuitem" :disabled="context.kind === 'web'" @click="closeContext(true)">关闭其他模块页签</button>
    <button role="menuitem" @click="context = null">取消</button>
  </div>
</template>

<style scoped>
.tab-actions { display:flex; flex:none; gap:8px; padding:4px 8px; background:#f2f4f6; }
.unified {
  display: flex;
  align-items: center;
  gap: 3px;
  padding: 3px 6px 0;
  background: #eef1f6;
  border-bottom: 1px solid #e5e6eb;
  flex-shrink: 0;
  overflow-x: auto;
  min-height: 32px;
}
.titlebar-drag { flex: 1; min-width: 24px; align-self: stretch; cursor: default; }
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
