<script setup lang="ts">
import { ref, watch, nextTick } from "vue";
import { useLayoutStore } from "../../stores/useLayoutStore";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import { useBrowserStore } from "../../stores/useBrowserStore";
import BrowserHost from "../browser/BrowserHost.vue";
import TabBar from "../browser/TabBar.vue";
import GridToolbar from "../browser/GridToolbar.vue";
import FileEditor from "../workspace/FileEditor.vue";

import FilePanel from "../workspace/FilePanel.vue";
import ArtifactPanel from "../workspace/ArtifactPanel.vue";
import ClipboardPanel from "../system/ClipboardPanel.vue";
import AuditPanel from "../workspace/AuditPanel.vue";
import RepoPanel from "../workspace/RepoPanel.vue";
import AppPanel from "../system/AppPanel.vue";
import TerminalPane from "../system/TerminalPane.vue";
import HomePanel from "../home/HomePanel.vue";

const layout = useLayoutStore();
const ws = useWorkspaceStore();
const browser = useBrowserStore();

// 目录模式「最近访问」下拉数据：localStorage 持久化最近浏览的目录
const RECENT_DIRS_KEY = "browser-os-recent-dirs";
const recentDirs = ref<string[]>(loadRecentDirs());
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
function onDirQuick(e: Event) {
  const sel = e.target as HTMLSelectElement;
  const p = sel.value;
  sel.value = "";
  if (!p) return;
  browser.url = p;
  openDirInDock();
}

// 精简模式切换后，地址栏显隐导致 viewport 尺寸变化，需重新定位子 webview
watch(
  () => layout.compactMode,
  () => nextTick(() => browser.relocate())
);

// 地址栏「前往/浏览」：按模式分发 —— 网址走浏览器，目录默认在中央主区展示（类网页）
function onAddrGo() {
  if (layout.addrMode === "dir") {
    openDirCenter();
  } else {
    browser.openBrowser();
  }
}

// 目录模式：默认在中央主区展示文件（与网页同一套展示区域）
async function openDirCenter() {
  const p = browser.url.trim();
  if (!p) {
    layout.showToast("请输入目录路径");
    return;
  }
  layout.browserDockOpen = false;
  layout.setView("files");
  layout.leftTab = "files";
  await ws.enterDir(p);
  recordRecentDir(p);
  layout.showToast("📁 " + p);
}

// 目录模式：默认在右侧 Dock 的文件面板显示（保持网页在左）
async function openDirInDock() {
  const p = browser.url.trim();
  if (!p) {
    layout.showToast("请输入目录路径");
    return;
  }
  layout.browserDockTab = "files";
  layout.browserDockOpen = true;
  await ws.enterDir(p);
  recordRecentDir(p);
  layout.showToast("📁 已在右侧打开目录: " + p);
}

</script>

<template>
  <main class="main">
    <!-- ===== 主页（快捷图标墙：网页/应用，可自定义编辑） ===== -->
    <div v-if="layout.mainView === 'home'" class="modview">
      <HomePanel />
    </div>

    <!-- ===== 浏览器主视图（含地址栏+页签+视口，对应 prototype 浏览器视图） ===== -->
    <template v-else-if="layout.mainView === 'browser' || layout.mainView === 'grid'">
      <div v-show="!layout.compactMode" class="addrbar">
        <button
          class="nav mode-btn"
          :class="{ active: layout.addrMode === 'dir' }"
          @click="layout.toggleAddrMode"
          :title="layout.addrMode === 'url' ? '当前：网址模式，点击切到目录模式' : '当前：目录模式，点击切到网址模式'"
        >{{ layout.addrMode === "url" ? "🌐" : "📁" }}</button>
        <button class="nav" @click="browser.goBack" title="后退">←</button>
        <button class="nav" @click="browser.goForward" title="前进">→</button>
        <button class="nav" @click="browser.reloadActive" title="刷新">⟳</button>
        <input
          v-model="browser.url"
          :placeholder="layout.addrMode === 'url' ? '输入网址或搜索词（如 baidu.com、天气）' : '输入目录路径，回车浏览（如 /home/you/Documents）'"
          @keyup.enter="onAddrGo"
        />
        <button class="go" @click="onAddrGo">{{ layout.addrMode === "url" ? "前往" : "浏览" }}</button>
        <!-- 目录模式：最近访问 / 常用目录快捷下拉 -->
        <select
          v-if="layout.addrMode === 'dir'"
          class="dir-quick"
          title="最近访问 / 常用目录"
          @change="onDirQuick"
        >
          <option value="">🕒 最近/常用</option>
          <optgroup label="最近访问" v-if="recentDirs.length">
            <option v-for="d in recentDirs" :key="d" :value="d">📁 {{ d }}</option>
          </optgroup>
          <optgroup label="常用目录">
            <option v-for="d in ws.startDirs" :key="d.path" :value="d.path">{{ d.name }}</option>
          </optgroup>
        </select>
        <button
          v-if="layout.addrMode === 'dir'"
          class="nav"
          @click="openDirInDock"
          title="在右侧 Dock 显示该目录（保持网页在左）"
        >🗂</button>
        <button class="nav" @click="layout.toggleBrowserDock('files')" title="边浏览边管理文件">🗂</button>
        <button class="nav" @click="layout.toggleBrowserDock('term')" title="边浏览边开终端">💻</button>
        <button class="nav" @click="layout.toggleCompact" title="精简模式：隐藏地址栏给网页更大空间">⛶</button>
      </div>
      <TabBar v-show="layout.mainView === 'browser' && !layout.compactMode" />
      <GridToolbar />
      <!-- 精简模式悬浮按钮：点击退出精简，恢复地址栏 -->
      <button
        v-if="layout.compactMode && layout.mainView === 'browser'"
        class="compact-exit"
        @click="layout.toggleCompact"
        title="退出精简模式"
      >☰</button>
      <div class="browser-body">
        <div class="viewport" :class="{ 'grid-mode': browser.gridOpen }">
          <!-- BrowserHost 在 browser/grid 视图都要参与布局（有 rect 供宫格定位），
               其内部用 visibility 控制显隐（isBrowserVisible），不能用 v-show=display:none，
               否则 grid 视图 rect=0 导致宫格定位全跳过、激活页签不移出。 -->
          <BrowserHost v-show="layout.mainView === 'browser' || layout.mainView === 'grid'" />
          <!-- 宫格每格标题栏覆盖层（序号+网址+关闭，相对 viewport 定位，与宫格坐标同步） -->
          <div v-if="browser.gridOpen" class="grid-close-layer">
            <div
              v-for="(rect, i) in browser.gridRects"
              :key="i"
              class="grid-cell-bar"
              :style="{ left: rect.x + 'px', top: rect.y + 'px', width: rect.w + 'px' }"
            >
              <span class="gcb-idx">{{ i + 1 }}</span>
              <span class="gcb-url" :title="browser.gridUrls[i]">{{ browser.gridUrls[i] || "未设置" }}</span>
              <button
                class="gcb-close"
                :title="`关闭宫格 ${i + 1}`"
                @click="browser.closeGridOne(i)"
              >✕</button>
            </div>
          </div>
        </div>
        <!-- 右侧 Dock：浏览网页的同时操作文件管理 / 终端 -->
        <aside v-if="layout.browserDockOpen && layout.mainView === 'browser'" class="browser-dock">
          <div class="tabs">
            <button :class="{ active: layout.browserDockTab === 'files' }" @click="layout.browserDockTab = 'files'">📂 文件</button>
            <button :class="{ active: layout.browserDockTab === 'term' }" @click="layout.browserDockTab = 'term'">💻 终端</button>
            <button class="close" @click="layout.browserDockOpen = false" title="收起">✕</button>
          </div>
          <FilePanel v-if="layout.browserDockTab === 'files'" />
          <!-- .terminal 是 absolute inset:0，需相对定位容器约束在 tab 栏之下 -->
          <div v-else class="dock-term-wrap"><TerminalPane /></div>
        </aside>
      </div>
    </template>

    <!-- ===== 文件 / 成果（共用带 Tab 的模块视图） ===== -->
    <div v-else-if="layout.mainView === 'files' || layout.mainView === 'arts'" class="modview">
      <div class="tabs">
        <button :class="{ active: layout.leftTab === 'files' }" @click="layout.leftTab = 'files'">📂 文件</button>
        <button :class="{ active: layout.leftTab === 'artifacts' }" @click="layout.leftTab = 'artifacts'">📦 成果</button>
      </div>
      <FilePanel v-if="layout.leftTab === 'files'" />
      <ArtifactPanel v-else />
    </div>

    <!-- ===== 剪贴板 ===== -->
    <div v-else-if="layout.mainView === 'clip'" class="modview">
      <div class="tabs"><button class="active">📋 剪贴板</button></div>
      <ClipboardPanel />
    </div>

    <!-- ===== 自有仓库（含 Git 两步闸门） ===== -->
    <div v-else-if="layout.mainView === 'repo'" class="modview">
      <div class="tabs"><button class="active">🛰️ 自有仓库</button></div>
      <RepoPanel />
    </div>

    <!-- ===== 系统应用 ===== -->
    <div v-else-if="layout.mainView === 'apps'" class="modview">
      <div class="tabs"><button class="active">🚀 系统应用</button></div>
      <AppPanel />
    </div>

    <!-- ===== 审计日志 ===== -->
    <div v-else-if="layout.mainView === 'audit'" class="modview">
      <div class="tabs"><button class="active">🛡️ 审计日志</button></div>
      <AuditPanel />
    </div>

    <!-- ===== 终端（全屏模块视图） ===== -->
    <div v-else-if="layout.mainView === 'term'" class="modview term-mod">
      <TerminalPane />
    </div>

    <!-- ===== 文件编辑器 / Markdown 预览（覆盖层） ===== -->
    <FileEditor v-else-if="layout.mainView === 'editor'" />
  </main>
</template>
