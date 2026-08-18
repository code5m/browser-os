<script setup lang="ts">
import { useLayoutStore } from "../../stores/useLayoutStore";
import { useBrowserStore } from "../../stores/useBrowserStore";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import { useSystemStore } from "../../stores/useSystemStore";

// 统一页签条：浏览器网页页签 + 目录页签 + 模块页签（终端/文件等）混排在同一条，
// 不区分类型 —— 点击即切换对应视图，行为与浏览器标签一致。
const layout = useLayoutStore();
const browser = useBrowserStore();
const ws = useWorkspaceStore();
const system = useSystemStore();

async function activateWeb(id: string) {
  if (browser.gridOpen) await browser.closeGridAll();
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
  layout.activateModTab(t.id);
  if (t.view === "apps") system.loadApps();
  if (t.path) ws.enterDir(t.path);
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
      :title="t.path || t.label"
    >
      <span class="tab-ic">{{ t.icon }}</span>
      <span class="tab-title">{{ t.label }}</span>
      <button class="tab-close" @click.stop="layout.closeModTab(t.id)" title="关闭">✕</button>
    </div>
    <button class="tab-new" @click="browser.tabNew()" title="新建页签">＋</button>
  </div>
</template>

<style scoped>
.unified {
  display: flex;
  align-items: center;
  gap: 3px;
  padding: 3px 6px 0;
  background: #eef1f6;
  border-bottom: 1px solid #e5e6eb;
  flex-shrink: 0;
  overflow-x: auto;
}
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
