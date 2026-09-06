<script setup lang="ts">
import { computed, ref, watch, nextTick, defineAsyncComponent } from "vue";
import { useLayoutStore } from "../../stores/useLayoutStore";
import { useBrowserStore } from "../../stores/useBrowserStore";
import { useBookmarkStore } from "../../stores/useBookmarkStore";
import BrowserHost from "../browser/BrowserHost.vue";
import BookmarkPanel from "../browser/BookmarkPanel.vue";
import ResourceWaterfall from "../browser/ResourceWaterfall.vue";
import SessionPanel from "../browser/SessionPanel.vue";
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
import ScriptPanel from "../workspace/ScriptPanel.vue";
import ToolBox from "../workspace/ToolBox.vue";
import CommandSnippetPanel from "../workspace/CommandSnippetPanel.vue";
// M4-4 数据库面板：懒加载（defineAsyncComponent），将其 15KB+ 纯逻辑(dbUi.ts)、
// store(useDatabaseStore.ts) 与组件从主 chunk 拆出，压低首屏 JS 体积（IF-2 构建体积闸门）。
// 仅在 mainView==='db' 首次渲染时才拉取该 chunk，不破坏其它视图。
// M4-8 定时任务面板同样懒加载：其纯逻辑(taskUi.ts)、store(useTaskStore.ts) 一并拆出主 chunk。
const TaskPanel = defineAsyncComponent(() => import("../workspace/TaskPanel.vue"));
const DatabasePanel = defineAsyncComponent(() => import("../workspace/DatabasePanel.vue"));
const SkillManagerPanel = defineAsyncComponent(() => import("../workspace/SkillManagerPanel.vue"));
const AgentManagerPanel = defineAsyncComponent(() => import("../workspace/AgentManagerPanel.vue"));
// M5-9 图谱面板：懒加载（defineAsyncComponent），将其纯逻辑(graphUi.ts)、store(useGraphStore.ts)
// 与组件从主 chunk 拆出，压低首屏 JS 体积（IF-2 构建体积闸门）。
const GraphPanel = defineAsyncComponent(() => import("../graph/GraphPanel.vue"));

const layout = useLayoutStore();
const browser = useBrowserStore();
const bookmarks = useBookmarkStore();

// 收藏夹侧栏只在浏览器视图展开（宫格视图定位链路更敏感，不纳入本次改动范围）
const bmPanelOpen = computed(() => bookmarks.panelOpen && layout.mainView === "browser");

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

// AI 模式开关会改变 browser-body 布局（底部多一条统一输入框，viewport 变矮），
// 宫格是原生窗口压在 HTML 之上，必须立即强制重排把输入框区域让出来
watch(
  () => browser.gridMode,
  () => nextTick(() => browser.forceGridRelayout())
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
      <div class="browser-body">
        <!-- M1-3 收藏夹侧栏：位于 viewport 左侧，撑窄 viewport 后由 BrowserHost 的
             ResizeObserver 自动重定位子 webview，无需手动 relocate -->
        <BookmarkPanel v-if="bmPanelOpen" />
        <div class="viewport" :class="{ 'grid-mode': browser.gridOpen }">
          <!-- BrowserHost 在 browser/grid 视图都要参与布局（有 rect 供宫格定位），
               其内部用 visibility 控制显隐（isBrowserVisible），不能用 v-show=display:none，
               否则 grid 视图 rect=0 导致宫格定位全跳过、激活页签不移出。 -->
          <BrowserHost v-show="layout.mainView === 'browser' || layout.mainView === 'grid'" />
        </div>
        <!-- 右侧 Dock：浏览网页的同时操作文件管理 / 终端 -->
        <aside v-if="layout.browserDockOpen && layout.mainView === 'browser'" class="browser-dock">
          <div class="tabs">
            <button :class="{ active: layout.browserDockTab === 'files' }" @click="layout.browserDockTab = 'files'">📂 文件</button>
            <button :class="{ active: layout.browserDockTab === 'term' }" @click="layout.browserDockTab = 'term'">💻 终端</button>
            <button :class="{ active: layout.browserDockTab === 'net' }" @click="layout.browserDockTab = 'net'">🌊 资源</button>
            <button :class="{ active: layout.browserDockTab === 'session' }" @click="layout.browserDockTab = 'session'">💾 会话</button>
            <button class="close" @click="layout.browserDockOpen = false" title="收起">✕</button>
          </div>
          <FilePanel v-if="layout.browserDockTab === 'files'" />
          <!-- M1-8 资源瀑布：请求/响应列表（脱敏 DTO），挂 Dock 第三 Tab -->
          <ResourceWaterfall v-else-if="layout.browserDockTab === 'net'" />
          <!-- M1-9 历史会话：保存/回看/恢复/删除，挂 Dock 第四 Tab -->
          <SessionPanel v-else-if="layout.browserDockTab === 'session'" />
          <!-- .terminal 是 absolute inset:0，需相对定位容器约束在 tab 栏之下 -->
          <div v-else class="dock-term-wrap"><TerminalPane /></div>
        </aside>
      </div>
    </template>

    <!-- ===== 文件（IDE 布局：左文件夹树，右预览/编辑） ===== -->
    <div v-else-if="layout.mainView === 'files'" class="modview">
      <FilePanel :ide="true" />
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

    <!-- ===== 脚本库（M2-5.a） ===== -->
    <div v-else-if="layout.mainView === 'scripts'" class="modview">
      <ScriptPanel />
    </div>

    <!-- ===== 命令库（M2-6.d） ===== -->
    <div v-else-if="layout.mainView === 'commands'" class="modview">
      <CommandSnippetPanel />
    </div>

    <!-- ===== 工具箱（M2-8） ===== -->
    <div v-else-if="layout.mainView === 'tools'" class="modview">
      <ToolBox />
    </div>

    <!-- ===== 数据库（M4-4） ===== -->
    <div v-else-if="layout.mainView === 'db'" class="modview">
      <DatabasePanel />
    </div>

    <!-- ===== 定时任务（M4-8） ===== -->
    <div v-else-if="layout.mainView === 'tasks'" class="modview">
      <TaskPanel />
    </div>
    <div v-else-if="layout.mainView === 'skills'" class="modview">
      <SkillManagerPanel />
    </div>
    <div v-else-if="layout.mainView === 'agents'" class="modview">
      <AgentManagerPanel />
    </div>
    <div v-else-if="layout.mainView === 'graph'" class="modview">
      <GraphPanel />
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
