<script setup lang="ts">
import { ref, watch, nextTick } from "vue";
import { useLayoutStore } from "../../stores/useLayoutStore";
import { useBrowserStore } from "../../stores/useBrowserStore";
import BrowserHost from "../browser/BrowserHost.vue";
import UnifiedTabBar from "./UnifiedTabBar.vue";
import FileEditor from "../workspace/FileEditor.vue";

import FilePanel from "../workspace/FilePanel.vue";
import ArtifactPanel from "../workspace/ArtifactPanel.vue";
import ClipboardPanel from "../system/ClipboardPanel.vue";
import AuditPanel from "../workspace/AuditPanel.vue";
import RepoPanel from "../workspace/RepoPanel.vue";
import AppPanel from "../system/AppPanel.vue";
import TerminalPane from "../system/TerminalPane.vue";
import HomePanel from "../home/HomePanel.vue";
import SettingsPanel from "../system/SettingsPanel.vue";

const layout = useLayoutStore();
const browser = useBrowserStore();

// 终端只挂载一次：首次进入后保持存活，避免切视图销毁 xterm 导致内容丢失
const termMounted = ref(false);
watch(
  () => layout.mainView,
  (v) => {
    if (v === "term") termMounted.value = true;
  },
  { immediate: true }
);

// 精简模式切换后，工具栏显隐导致 viewport 尺寸变化，需重新定位子 webview
watch(
  () => layout.compactMode,
  () => nextTick(() => browser.relocate())
);

</script>

<template>
  <main class="main">
    <!-- 统一页签条：网页/目录/模块页签混排，所有视图常驻 -->
    <UnifiedTabBar v-show="!layout.compactMode" />

    <!-- ===== 主页（快捷图标墙：网页/应用，可自定义编辑） ===== -->
    <div v-if="layout.mainView === 'home'" class="modview">
      <HomePanel />
    </div>

    <!-- ===== 浏览器主视图（地址栏已合并进顶部全局工具栏，页签走统一页签条） ===== -->
    <template v-else-if="layout.mainView === 'browser' || layout.mainView === 'grid'">
      <!-- 精简模式悬浮按钮：点击退出精简，恢复工具栏 -->
      <button
        v-if="layout.compactMode && layout.mainView === 'browser'"
        class="compact-exit"
        @click="layout.toggleCompact"
        title="退出精简模式"
      >☰</button>
      <div class="browser-body" :class="{ 'grid-ai-mode': layout.mainView === 'grid' && browser.gridMode === 'ai' }">
        <div class="viewport" :class="{ 'grid-mode': browser.gridOpen }">
          <!-- BrowserHost 在 browser/grid 视图都要参与布局（有 rect 供宫格定位），
               其内部用 visibility 控制显隐（isBrowserVisible），不能用 v-show=display:none，
               否则 grid 视图 rect=0 导致宫格定位全跳过、激活页签不移出。 -->
          <BrowserHost v-show="layout.mainView === 'browser' || layout.mainView === 'grid'" />
        </div>
        <!-- AI 模式统一输入框：向所有宫格中的 AI 同时发送问题 -->
        <div v-if="layout.mainView === 'grid' && browser.gridMode === 'ai'" class="grid-ai-bar">
          <input
            v-model="browser.gridAiInput"
            placeholder="输入问题，同时发送给所有宫格中的 AI..."
            @keyup.enter="browser.gridSendAi"
          />
          <button class="primary" @click="browser.gridSendAi">发送</button>
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

    <!-- ===== 文件 ===== -->
    <div v-else-if="layout.mainView === 'files'" class="modview">
      <FilePanel />
    </div>

    <!-- ===== 知识库（原成果） ===== -->
    <div v-else-if="layout.mainView === 'arts'" class="modview">
      <ArtifactPanel />
    </div>

    <!-- ===== 剪贴板 ===== -->
    <div v-else-if="layout.mainView === 'clip'" class="modview">
      <ClipboardPanel />
    </div>

    <!-- ===== 自有仓库（含 Git 两步闸门） ===== -->
    <div v-else-if="layout.mainView === 'repo'" class="modview">
      <RepoPanel />
    </div>

    <!-- ===== 系统应用 ===== -->
    <div v-else-if="layout.mainView === 'apps'" class="modview">
      <AppPanel />
    </div>

    <!-- ===== 审计日志 ===== -->
    <div v-else-if="layout.mainView === 'audit'" class="modview">
      <AuditPanel />
    </div>

    <!-- ===== 系统设置 ===== -->
    <div v-else-if="layout.mainView === 'settings'" class="modview">
      <SettingsPanel />
    </div>

    <!-- ===== 终端（全屏模块视图，首次打开后保持挂载） ===== -->
    <div v-show="layout.mainView === 'term'" class="modview term-mod">
      <TerminalPane v-if="termMounted" />
    </div>

    <!-- ===== 文件编辑器 / Markdown 预览（覆盖层） ===== -->
    <FileEditor v-if="layout.mainView === 'editor'" />
  </main>
</template>
