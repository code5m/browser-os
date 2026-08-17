<script setup lang="ts">
import { useLayoutStore } from "../../stores/useLayoutStore";
import { useBrowserStore } from "../../stores/useBrowserStore";
import { useSystemStore } from "../../stores/useSystemStore";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";

const layout = useLayoutStore();
const browser = useBrowserStore();
const system = useSystemStore();
const ws = useWorkspaceStore();

// 对应 prototype.html 活动栏 9 模块
const items = [
  { view: "home", icon: "🏠", label: "主页" },
  { view: "browser", icon: "📁", label: "浏览" },
  { view: "files", icon: "📂", label: "文件" },
  { view: "clip", icon: "📋", label: "剪贴板" },
  { view: "arts", icon: "📚", label: "成果" },
  { view: "grid", icon: "🗂️", label: "宫格" },
  { view: "apps", icon: "🚀", label: "应用" },
  { view: "term", icon: "💻", label: "终端" },
  { view: "repo", icon: "🛰️", label: "仓库" },
  { view: "audit", icon: "🛡️", label: "审计" },
] as const;

function onItem(v: string) {
  if (v === "apps") system.loadApps();
  if (v === "grid") {
    layout.setView("grid");
    // 打开宫格工具条（含布局切换：宫格/四分/横向）
    layout.gridToolbarOpen = true;
    // 总是重建宫格：buildGrid 内部 createGrid 会先 close_grid 再重建（幂等），
    // 避免 gridOpen 标志与后端宫格 webview 实际状态脱节导致的"有工具条没宫格"。
    browser.buildGrid();
    return;
  }
  layout.setView(v as any);
}
</script>

<template>
  <nav class="activity">
    <button
      v-for="it in items"
      :key="it.view"
      :class="{ active: layout.mainView === it.view }"
      @click="onItem(it.view)"
      :title="it.label"
    >
      <span class="ic">{{ it.icon }}</span>
      <span class="lab">{{ it.label }}</span>
    </button>
      <span class="sep"></span>
    <button class="collect" :title="'采集选中内容'" @click="ws.collectSelection">
      <span class="ic">📥</span>
      <span class="lab">采集</span>
    </button>
    <button class="sys" :title="'设置'">
      <span class="ic">⚙️</span>
    </button>
  </nav>
</template>
