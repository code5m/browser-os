<script setup lang="ts">
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

const layout = useLayoutStore();
const ws = useWorkspaceStore();
const browser = useBrowserStore();
</script>

<template>
  <main class="main">
    <!-- ===== 浏览器主视图（含地址栏+页签+视口，对应 prototype 浏览器视图） ===== -->
    <template v-if="layout.mainView === 'browser' || layout.mainView === 'grid'">
      <div class="addrbar">
        <button class="nav" @click="browser.goBack" title="后退">←</button>
        <button class="nav" @click="browser.goForward" title="前进">→</button>
        <button class="nav" @click="browser.reloadActive" title="刷新">⟳</button>
        <input
          v-model="browser.url"
          placeholder="输入网址或搜索词（如 baidu.com、天气）"
          @keyup.enter="browser.openBrowser"
        />
        <button class="go" @click="browser.openBrowser">前往</button>
        <button class="nav" @click="layout.toggleBrowserDock('files')" title="边浏览边管理文件">🗂</button>
        <button class="nav" @click="layout.toggleBrowserDock('term')" title="边浏览边开终端">💻</button>
      </div>
      <TabBar v-show="layout.mainView === 'browser'" />
      <GridToolbar />
      <div class="browser-body">
        <div class="viewport">
          <BrowserHost v-show="layout.mainView === 'browser'" />
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
