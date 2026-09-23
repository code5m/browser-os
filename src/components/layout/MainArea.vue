<script setup lang="ts">
import { watch, nextTick, defineAsyncComponent, h, computed } from "vue";
import { useLayoutStore } from "../../stores/useLayoutStore";
import { useBrowserStore } from "../../capabilities/browser/public";
import { contributionRegistry } from "../../capability/contribution/registry";
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types";
import HomePanel from "../home/HomePanel.vue";
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

// Task 面板已升格为 capabilities/task/ 能力：经 WORKBENCH_MAIN 贡献（view='tasks'）懒加载注册，
// MainArea 只按槽渲染（见下方通用 viewOf 分支），不再静态 import 本能力内部（C3 关键）。
// 懒加载仍保留在 capabilities/task/index.ts 的 defineAsyncComponent 中。
// Database 面板已升格为 capabilities/database/ 能力：经 WORKBENCH_MAIN 贡献（view='db'）懒加载注册，
// MainArea 只按槽渲染（见下方通用 viewOf 分支），不再静态 import 本能力内部（C3 关键）。
// 懒加载仍保留在 capabilities/database/index.ts 的 defineAsyncComponent 中（15KB+ dbUi + store 拆主 chunk）。
// Skill 面板已升格为 capabilities/skill/ 能力：经 WORKBENCH_MAIN 贡献（view='skills'）懒加载注册，
// MainArea 只按槽渲染（见下方通用 viewOf 分支），不再静态 import 本能力内部（C3 关键）。
// Agent 面板已升格为 capabilities/agent/ 能力：经 WORKBENCH_MAIN 贡献（view='agents'）懒加载注册，
// MainArea 只按槽渲染（见下方通用 viewOf 分支），不再静态 import 本能力内部（C3 关键）。
// Graph 面板已升格为 capabilities/graph/ 能力：经 WORKBENCH_MAIN 贡献（view='graph'）懒加载注册，
// MainArea 只按槽渲染（见下方通用 viewOf 分支），不再静态 import 本能力内部（C3 关键）。
// 懒加载仍保留在 capabilities/graph/index.ts 的 defineAsyncComponent 中。
// Plugin 面板已升格为 capabilities/plugin/ 能力：经 WORKBENCH_MAIN 贡献（view='plugin'）懒加载注册，
// MainArea 只按槽渲染（见下方通用 viewOf 分支），不再静态 import 本能力内部（C3 关键）。
// 懒加载仍保留在 capabilities/plugin/index.ts 的 defineAsyncComponent 中（pluginUi + store 拆主 chunk）。

const layout = useLayoutStore();
const browser = useBrowserStore();

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

// ===== UI-4 DockContribution：Dock 页签改为「贡献驱动」，不再硬编码 =====
// 旧实现把 4 个页签（文件/终端/资源/会话）的图标、文案、顺序、可用 view 集合
// 全部写死在 Shell 模板里，导致：
//   1) 能力无法新增 Dock 页签（加了贡献也渲染不出来）；
//   2) Terminal absent 时「💻 终端」按钮仍在，点进去渲染空 —— 死页签。
// 改为按贡献渲染后：absent capability 的贡献根本不在表里 → 页签自动消失，无死页签。
// Shell 依旧不知道任何能力名，只认 slot + view + 展示元数据。
const dockTabs = contributionRegistry
  .getDockTabContributions(CONTRIBUTION_SLOTS.BROWSER_DOCK)
  .map((c) => ({
    id: c.id,
    view: c.view as string,
    // 未声明展示元数据时退化为 view 名，保证渲染不炸（不引入 undefined 文案）
    label: c.label ?? c.view ?? "",
    icon: c.icon ?? "",
  }));

// 当前选中页签：若持久化的 tab 已不存在（能力缺席/被卸载），回退到第一个可用页签。
// full profile 下 files 恒在 → 行为与迁移前完全一致。
const activeDockTab = computed(() => {
  const t = layout.browserDockTab;
  return dockTabs.some((d) => d.view === t) ? t : (dockTabs[0]?.view ?? "");
});

// 常驻主视图贡献：Shell 只按槽渲染，显隐由能力组件自管（如终端保持 xterm 挂载）。
// Terminal absent → 槽为空 → 不渲染任何东西，更不会有 PTY 出生点（C3 ABSENT 关键）。
const residentMainContributions = contributionRegistry.getSurfaceContributions(
  CONTRIBUTION_SLOTS.WORKBENCH_MAIN_RESIDENT,
);
// Browser 原生宿主：经 CONTRIBUTION_SLOTS.BROWSER_HOST 渲染；Browser absent → undefined → 不创建 webview。
const browserHostComp = contributionRegistry.getSurfaceContributions(
  CONTRIBUTION_SLOTS.BROWSER_HOST,
)[0]?.component;

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

    <!-- ===== 常驻主视图贡献（能力自管显隐；Shell 只按槽渲染、不按 view 卸载） =====
         例：终端 xterm 切走不得卸载，故其主视图走本槽。槽为空 → 不渲染任何东西。 -->
    <component v-for="c in residentMainContributions" :key="c.id" :is="c.component" />

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
            <!-- UI-4：页签来自能力贡献（顺序/图标/文案由贡献声明，与迁移前一致） -->
            <button
              v-for="t in dockTabs"
              :key="t.id"
              :class="{ active: layout.browserDockTab === t.view }"
              @click="layout.browserDockTab = t.view"
            >{{ t.icon + " " + t.label }}</button>
            <button class="close" @click="layout.browserDockOpen = false" title="收起">✕</button>
          </div>
          <!-- UI-4：单一动态渲染点。Dock 面板仍由能力提供（文件/终端/资源/会话）。
               Terminal absent → term 贡献不在 → 页签消失、此处无 PTY 出生点。 -->
          <component :is="dockOf(activeDockTab)" v-if="activeDockTab" />
        </aside>
      </div>
    </template>

    <!-- ===== 工作区主视图（Files/Arts/Repo/Scripts/Commands/Audit）：经通用 Contribution 按 view 渲染 ===== -->
    <div v-else-if="viewOf(layout.mainView)" class="modview">
      <component :is="viewOf(layout.mainView)" />
    </div>

    <!-- ===== 剪贴板：已升格 capabilities/clipboard/，经 WORKBENCH_MAIN 贡献 view='clip' 渲染（由通用 viewOf 分支处理） ===== -->
    <!-- ===== 系统应用：已升格 capabilities/apps/，经 WORKBENCH_MAIN 贡献 view='apps' 渲染（由通用 viewOf 分支处理） ===== -->
    <!-- ===== 工具箱：已升格 capabilities/tools/，经 WORKBENCH_MAIN 贡献 view='tools' 渲染（由通用 viewOf 分支处理） ===== -->

    <!-- ===== 数据库（M4-4）：已升格 capabilities/database/，经 WORKBENCH_MAIN 贡献 view='db' 渲染（由上方通用 viewOf 分支处理） ===== -->

    <!-- ===== 笔记库（Vault，STAGE I-A）：已升格 capabilities/vault/，经 WORKBENCH_MAIN 贡献 view='vault' 渲染（由上方通用 viewOf 分支处理） ===== -->
    <!-- ===== 定时任务（M4-8）：已升格 capabilities/task/，经 WORKBENCH_MAIN 贡献 view='tasks' 渲染（由上方通用 viewOf 分支处理） ===== -->

    <!-- ===== 插件管理器（M5-W14）：已升格 capabilities/plugin/，经 WORKBENCH_MAIN 贡献 view='plugin' 渲染（由上方通用 viewOf 分支处理） ===== -->
    <!-- ===== 技能（Skill）：已升格 capabilities/skill/，经 WORKBENCH_MAIN 贡献 view='skills' 渲染（由通用 viewOf 分支处理） ===== -->
    <!-- ===== 智能体（Agent）：已升格 capabilities/agent/，经 WORKBENCH_MAIN 贡献 view='agents' 渲染（由通用 viewOf 分支处理） ===== -->
    <!-- ===== 知识图谱（M5-9）：已升格 capabilities/graph/，经 WORKBENCH_MAIN 贡献 view='graph' 渲染（由上方通用 viewOf 分支处理） ===== -->

    <!-- ===== 系统设置（STAGE I-C）：已升格 src/settings/，经 WORKBENCH_MAIN 贡献 view='settings' 渲染（由上方通用 viewOf 分支处理） ===== -->

    <!-- ===== 终端主视图：已迁入 Terminal 能力（workbench-main-resident 槽），
         由 capabilities/terminal/ui/TerminalView.vue 自管挂载/显隐与 PTY 出生点。 -->

    <!-- ===== 文件编辑器 / Markdown 预览（覆盖层）：经贡献渲染。
         仅在编辑视图激活(mainView==='editor')时挂载——否则 home/其它视图下会与主区并列、
         作为块级元素落到下方占半屏（既有布局 bug：编辑器贡献常驻却未判激活态）。
         Shell 仍只按槽渲染、零能力专属知识；关闭由 closeEditor() 切走视图触发。 -->
    <component :is="viewOf('editor')" v-if="viewOf('editor') && layout.mainView === 'editor'" />
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
