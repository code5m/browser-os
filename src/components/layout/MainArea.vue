<script setup lang="ts">
import { computed, ref, watch, nextTick, defineAsyncComponent, h } from "vue";
import { useLayoutStore } from "../../stores/useLayoutStore";
import { useBrowserStore } from "../../capabilities/browser/public";
import { contributionRegistry } from "../../capability/contribution/registry";
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types";
import VaultPanel from '../workspace/VaultPanel.vue';
import ClipboardPanel from "../system/ClipboardPanel.vue";
import AppPanel from "../system/AppPanel.vue";
import TerminalPane from "../system/TerminalPane.vue";
import { useSystemStore } from "../../stores/useSystemStore";
import HomePanel from "../home/HomePanel.vue";
import SettingsPanel from "../system/SettingsPanel.vue";
import ToolBox from "../workspace/ToolBox.vue";
// M4-4 数据库面板：懒加载（defineAsyncComponent），将其 15KB+ 纯逻辑(dbUi.ts)、
// store(useDatabaseStore.ts) 与组件从主 chunk 拆出，压低首屏 JS 体积（IF-2 构建体积闸门）。
// 仅在 mainView==='db' 首次渲染时才拉取该 chunk，不破坏其它视图。
// M4-8 定时任务面板同样懒加载：其纯逻辑(taskUi.ts)、store(useTaskStore.ts) 一并拆出主 chunk。
// W17(A7): 异步面板加载中/失败态兜底组件（纯展示，不引入运行时行为；用 h() 而非 template 以避免运行时编译依赖）。
const panelLoading = {
  render: () =>
    h("div", { class: "modview panel-state", role: "status", "aria-live": "polite" }, "面板加载中…"),
};
const panelError = {
  render: () =>
    h("div", { class: "modview panel-state panel-error", role: "alert" }, "该面板暂时无法显示"),
};

// M4-8 定时任务面板同样懒加载：其纯逻辑(taskUi.ts)、store(useTaskStore.ts) 一并拆出主 chunk。
const TaskPanel = defineAsyncComponent({
  loader: () => import("../workspace/TaskPanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
});
// M4-4 数据库面板：懒加载（defineAsyncComponent），将其 15KB+ 纯逻辑(dbUi.ts)、
// store(useDatabaseStore.ts) 与组件从主 chunk 拆出，压低首屏 JS 体积（IF-2 构建体积闸门）。
// 仅在 mainView==='db' 首次渲染时才拉取该 chunk，不破坏其它视图。
const DatabasePanel = defineAsyncComponent({
  loader: () => import("../workspace/DatabasePanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
});
const SkillManagerPanel = defineAsyncComponent({
  loader: () => import("../workspace/SkillManagerPanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
});
const AgentManagerPanel = defineAsyncComponent({
  loader: () => import("../workspace/AgentManagerPanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
});
// M5-9 图谱面板：懒加载（defineAsyncComponent），将其纯逻辑(graphUi.ts)、store(useGraphStore.ts)
// 与组件从主 chunk 拆出，压低首屏 JS 体积（IF-2 构建体积闸门）。
const GraphPanel = defineAsyncComponent({
  loader: () => import("../graph/GraphPanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
});
// M5-W14 插件管理器面板：懒加载（defineAsyncComponent），将其纯逻辑(pluginUi.ts)、
// store(usePluginStore.ts) 与组件从主 chunk 拆出，压低首屏 JS 体积（IF-2 构建体积闸门）。
const PluginManager = defineAsyncComponent({
  loader: () => import("../plugin/PluginManager.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
});

const layout = useLayoutStore();
const browser = useBrowserStore();
const system = useSystemStore();

// 收藏夹等「浏览器侧栏」贡献：经通用 Contribution Registry 按 slot 遍历渲染。
// Shell 不持有 Bookmark 专属知识（不 import 其 store / ui），C3 关键（8B.1）。
const sidebarContributions = contributionRegistry.getSurfaceContributions(
  CONTRIBUTION_SLOTS.BROWSER_SIDEBAR,
);

// Workspace 主视图 / Dock 贡献：经通用 Contribution Registry 按 view 认领渲染。
// Shell 不持有 Workspace 专属知识（不 import 其 store / ui），C3 关键（Train B）。
const workbenchMainContributions = contributionRegistry.getSurfaceContributions(
  CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
);
function viewOf(view: string) {
  return workbenchMainContributions.find((c) => c.view === view)?.component;
}
const browserDockContributions = contributionRegistry.getSurfaceContributions(
  CONTRIBUTION_SLOTS.BROWSER_DOCK,
);
function dockOf(view: string) {
  return browserDockContributions.find((c) => c.view === view)?.component;
}
// Browser 原生宿主：经 CONTRIBUTION_SLOTS.BROWSER_HOST 渲染；Browser absent → undefined → 不创建 webview。
const browserHostComp = contributionRegistry.getSurfaceContributions(
  CONTRIBUTION_SLOTS.BROWSER_HOST,
)[0]?.component;

// 终端宫格布局 class（单 / 2 / 4 / 9）
const gridClass = computed(() => {
  if (!system.termGrid) return "grid-off";
  return "grid-on cols-" + system.termGridCount;
});

// 进入终端视图时确保至少有一个终端实例（多实例宫格共享 termPanes）
watch(
  () => layout.mainView,
  (v) => {
    if (v === "term") system.ensureTerm();
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
        <!-- M1-3 收藏夹侧栏：经通用 Contribution Registry 按 slot 渲染（Shell 零能力专属知识）。
             可见性由贡献组件自身按 panelOpen 控制（能力包内）。 -->
        <template v-for="c in sidebarContributions" :key="c.id">
          <component :is="c.component" />
        </template>
        <div class="viewport" :class="{ 'grid-mode': browser.gridOpen }">
          <!-- BrowserHost 在 browser/grid 视图都要参与布局（有 rect 供宫格定位），
               其内部用 visibility 控制显隐（isBrowserVisible），不能用 v-show=display:none，
               否则 grid 视图 rect=0 导致宫格定位全跳过、激活页签不移出。 -->
          <component :is="browserHostComp" v-show="layout.mainView === 'browser' || layout.mainView === 'grid'" />
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
          <component :is="dockOf('files')" v-if="layout.browserDockTab === 'files'" />
          <!-- M1-8 资源瀑布：请求/响应列表（脱敏 DTO），挂 Dock 第三 Tab -->
          <component :is="dockOf('net')" v-else-if="layout.browserDockTab === 'net'" />
          <!-- M1-9 历史会话：保存/回看/恢复/删除，挂 Dock 第四 Tab -->
          <component :is="dockOf('session')" v-else-if="layout.browserDockTab === 'session'" />
          <!-- 浏览时右侧 Dock 终端：与终端视图共享同一组实例（dock 内竖向堆叠） -->
          <div v-else class="dock-term-wrap">
            <div class="term-grid dock-grid" v-if="system.termPanes.length">
              <TerminalPane v-for="pane in system.termPanes" :key="pane.id" :paneId="pane.id" small />
            </div>
            <div v-else class="term-empty">
              <button @click="system.addTermPane()">＋ 新建终端</button>
            </div>
          </div>
        </aside>
      </div>
    </template>

    <!-- ===== 工作区主视图（Files/Arts/Repo/Scripts/Commands/Audit）：经通用 Contribution 按 view 渲染 ===== -->
    <div v-else-if="viewOf(layout.mainView)" class="modview">
      <component :is="viewOf(layout.mainView)" />
    </div>

    <!-- ===== 剪贴板 ===== -->
    <div v-else-if="layout.mainView === 'clip'" class="modview">
      <ClipboardPanel />
    </div>

    <!-- ===== 系统应用 ===== -->
    <div v-else-if="layout.mainView === 'apps'" class="modview">
      <AppPanel />
    </div>

    <!-- ===== 工具箱（M2-8） ===== -->
    <div v-else-if="layout.mainView === 'tools'" class="modview">
      <ToolBox />
    </div>

    <!-- ===== 数据库（M4-4） ===== -->
    <div v-else-if="layout.mainView === 'db'" class="modview">
      <DatabasePanel />
    </div>

    <div v-else-if="layout.mainView === 'vault'" class="modview"><VaultPanel /></div>
    <!-- ===== 定时任务（M4-8） ===== -->
    <div v-else-if="layout.mainView === 'tasks'" class="modview">
      <TaskPanel />
    </div>

    <!-- ===== 插件管理器（M5-W14） ===== -->
    <div v-else-if="layout.mainView === 'plugin'" class="modview">
      <PluginManager />
    </div>
    <div v-else-if="layout.mainView === 'skills'" class="modview">
      <SkillManagerPanel />
    </div>
    <div v-else-if="layout.mainView === 'agents'" class="modview">
      <AgentManagerPanel />
    </div>
    <div v-else-if="layout.mainView === 'graph'" class="modview" role="region" aria-label="知识图谱">
      <GraphPanel />
    </div>

    <!-- ===== 系统设置 ===== -->
    <div v-else-if="layout.mainView === 'settings'" class="modview">
      <SettingsPanel />
    </div>

    <!-- ===== 终端（全屏模块视图，多实例宫格） ===== -->
    <div v-show="layout.mainView === 'term'" class="modview term-mod">
      <div class="term-toolbar">
        <span class="term-toolbar-title">终端</span>
        <button :class="{ active: !system.termGrid }" @click="system.setTermGrid(false)">单</button>
        <button :class="{ active: system.termGrid && system.termGridCount === 2 }" @click="system.setTermGridCount(2); system.setTermGrid(true)">2</button>
        <button :class="{ active: system.termGrid && system.termGridCount === 4 }" @click="system.setTermGridCount(4); system.setTermGrid(true)">4</button>
        <button :class="{ active: system.termGrid && system.termGridCount === 9 }" @click="system.setTermGridCount(9); system.setTermGrid(true)">9</button>
        <span class="term-toolbar-spacer"></span>
        <button @click="system.addTermPane()" title="新建终端（追加一个实例）">＋ 终端</button>
      </div>
      <div class="term-grid" :class="gridClass" v-if="system.termPanes.length">
        <TerminalPane v-for="pane in system.termPanes" :key="pane.id" :paneId="pane.id" />
      </div>
      <div v-else class="term-empty">
        <p>还没有终端</p>
        <button @click="system.addTermPane()">＋ 新建终端</button>
      </div>
    </div>

    <!-- ===== 文件编辑器 / Markdown 预览（覆盖层）：经贡献渲染 ===== -->
    <component :is="viewOf('editor')" v-if="viewOf('editor')" />
    <!-- ===== W17(A7) 兜底：未知/空视图时主区不得空白或死区 ===== -->
    <!-- This is intentionally independent from FileEditor; an adjacent v-else
         would bind to the editor v-if and render during every normal view. -->
    <div
      v-if="![
        'home', 'browser', 'grid', 'files', 'arts', 'clip', 'repo', 'apps', 'audit',
        'scripts', 'commands', 'tools', 'db', 'tasks', 'plugin', 'skills', 'agents',
        'graph', 'settings', 'term', 'editor', 'vault'
      ].includes(layout.mainView)"
      class="modview panel-state"
      role="alert"
      aria-live="polite"
    >
      <div>
        <div class="pf-title">当前视图不可用</div>
        <div class="pf-desc">未找到对应的面板（{{ layout.mainView || "空" }}）。请从左侧栏切换其它功能。</div>
      </div>
    </div>
  </main>
</template>

<style scoped>
.panel-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 100%;
  min-height: 200px;
  padding: 24px;
  color: #9aa4b2;
  font-size: 14px;
  text-align: center;
}
.term-mod {
  height: 100%;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.term-toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  background: #252526;
  border-bottom: 1px solid #1b1b1b;
  flex-shrink: 0;
}
.term-toolbar-title {
  color: #ccc;
  font-size: 12px;
  margin-right: 4px;
}
.term-toolbar-spacer {
  flex: 1;
}
.term-toolbar button {
  border: none;
  background: #3a3a3a;
  color: #ccc;
  cursor: pointer;
  padding: 2px 10px;
  border-radius: 3px;
  font-size: 12px;
  margin-left: 2px;
}
.term-toolbar button.active {
  background: #0e639c;
  color: #fff;
}
.term-toolbar button:hover {
  background: #4a4a4a;
}
.term-toolbar button.active:hover {
  background: #1177bb;
}
.term-grid {
  flex: 1;
  min-height: 0;
  display: grid;
  gap: 1px;
  padding: 1px;
  background: #252526;
}
.term-grid.grid-off {
  grid-template-columns: 1fr;
  grid-template-rows: 1fr;
}
.term-grid.grid-on.cols-2 {
  grid-template-columns: 1fr 1fr;
  grid-template-rows: 1fr;
}
.term-grid.grid-on.cols-4 {
  grid-template-columns: 1fr 1fr;
  grid-template-rows: 1fr 1fr;
}
.term-grid.grid-on.cols-9 {
  grid-template-columns: 1fr 1fr 1fr;
  grid-template-rows: 1fr 1fr 1fr;
}
.term-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: #9aa4b2;
}
.term-empty button {
  border: 1px solid #3a3a3a;
  background: #2d2d2d;
  color: #ccc;
  padding: 6px 14px;
  border-radius: 4px;
  cursor: pointer;
}
.term-empty button:hover {
  background: #3a3a3a;
}
/* Dock 内终端：竖向堆叠，避免在小空间挤成宫格 */
.dock-term-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.dock-term-wrap .term-grid.dock-grid {
  display: flex;
  flex-direction: column;
}
.dock-term-wrap .term-grid.dock-grid .terminal-xterm {
  min-height: 120px;
}
.panel-error {
  color: #ff7a7a;
}
.pf-title {
  font-size: 15px;
  font-weight: 600;
  color: #cbd5e0;
}
.pf-desc {
  max-width: 420px;
  line-height: 1.7;
}
</style>
