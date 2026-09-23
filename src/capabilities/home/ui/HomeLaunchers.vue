<script setup lang="ts">
import { useLayoutStore } from "../../../stores/useLayoutStore";
import type { MainView } from "../../../stores/useLayoutStore";
import { useBrowserStore } from "../../browser/public";
import { useAppsStore } from "../../apps/public";

const layout = useLayoutStore();
const browser = useBrowserStore();
const appsStore = useAppsStore();

// 主页启动器：只覆盖「主要工作区」入口，不含主页自身、编辑器覆盖层与设置。
//
// 宫格（grid）刻意不列入：它的打开需要 gridMode + buildGrid 的重建仪式
// （原生窗口会压在 HTML 之上），该仪式属 ActivityBar / UnifiedTabBar 职责。
// 主页再实现一遍会形成第二条路径，也与 W17「不新增运行时行为」冲突。
interface Launcher {
  view: MainView;
  icon: string;
  label: string;
}

const LAUNCHERS: Launcher[] = [
  { view: "browser", icon: "🌐", label: "浏览" },
  { view: "files", icon: "📂", label: "文件" },
  { view: "term", icon: "💻", label: "终端" },
  { view: "clip", icon: "📋", label: "剪贴板" },
  { view: "arts", icon: "📚", label: "知识库" },
  { view: "apps", icon: "🚀", label: "应用" },
  { view: "scripts", icon: "📜", label: "脚本库" },
  { view: "commands", icon: "⚡", label: "命令库" },
  { view: "tools", icon: "🧰", label: "工具箱" },
  { view: "db", icon: "🗄️", label: "数据库" },
  { view: "tasks", icon: "⏰", label: "定时任务" },
  { view: "skills", icon: "🛠️", label: "技能" },
  { view: "agents", icon: "🤖", label: "智能体" },
  { view: "graph", icon: "🕸️", label: "图谱" },
  { view: "plugin", icon: "🔌", label: "插件" },
  { view: "repo", icon: "🛰️", label: "仓库" },
  { view: "audit", icon: "🛡️", label: "审计" },
];

// 与 ActivityBar / UnifiedTabBar 保持同一套导航语义，避免主页另开一条：
//   1) 进入非宫格视图前先关掉宫格（原生窗口压在 HTML 之上，不关会残留）
//   2) 浏览器主视图不是模块页签，直接切视图
//   3) 应用列表按需加载后走模块页签（同视图去重复用，不会点出一排重复标签）
async function openArea(view: MainView) {
  if (view !== "grid" && browser.gridOpen) await browser.closeGrid();
  if (view === "browser") {
    layout.activateBrowser();
    return;
  }
  if (view === "apps") appsStore.loadApps();
  layout.openModule(view);
}
</script>

<template>
  <section class="home-section" aria-labelledby="home-launchers-title">
    <h2 id="home-launchers-title" class="home-section-title">主要工作区</h2>
    <div class="launch-grid">
      <button
        v-for="it in LAUNCHERS"
        :key="it.view"
        type="button"
        class="launch-card"
        :class="{ active: layout.mainView === it.view }"
        :aria-label="it.label"
        :aria-current="layout.mainView === it.view ? 'page' : undefined"
        :title="it.label"
        @click="openArea(it.view)"
      >
        <span class="launch-icon" aria-hidden="true">{{ it.icon }}</span>
        <span class="launch-label">{{ it.label }}</span>
      </button>
    </div>
  </section>
</template>

<style scoped>
.home-section {
  margin-bottom: 22px;
}
.home-section-title {
  margin: 0 0 10px;
  font-size: 13px;
  font-weight: 600;
  color: #4e5969;
  letter-spacing: 0.3px;
}
.launch-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
  gap: 10px;
}
.launch-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  background: #fff;
  border: 1px solid #e8ebf0;
  border-radius: 10px;
  padding: 12px 6px;
  cursor: pointer;
  font: inherit;
  color: inherit;
  transition: transform 0.15s, box-shadow 0.15s, border-color 0.15s;
}
.launch-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 14px rgba(43, 108, 176, 0.12);
  border-color: #c6d8ef;
}
.launch-card.active {
  border-color: #2b6cb0;
  box-shadow: 0 0 0 2px rgba(43, 108, 176, 0.12);
}
.launch-card:focus-visible {
  outline: 2px solid #2b6cb0;
  outline-offset: 2px;
}
.launch-icon {
  font-size: 20px;
  line-height: 1;
}
.launch-label {
  font-size: 12px;
  color: #4e5969;
}

/* 窄窗口：缩小栅格与字号，保证 17 个入口不挤成一团 */
@media (max-width: 720px) {
  .launch-grid {
    grid-template-columns: repeat(auto-fill, minmax(84px, 1fr));
    gap: 8px;
  }
  .launch-card {
    padding: 10px 4px;
  }
  .launch-icon {
    font-size: 18px;
  }
  .launch-label {
    font-size: 11px;
  }
}
</style>
