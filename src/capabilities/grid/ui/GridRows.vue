<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { ResourceStats } from "../../../types";
import { bridge } from "../../../bridge";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { useGridStore } from "../public";
import GridArchiveBar from "./GridArchiveBar.vue";
import { useGridHost } from "../composables/useGridHost";

const layout = useLayoutStore();
const grid = useGridStore();
const hostElement = ref<HTMLElement | null>(null);
useGridHost(hostElement);
onMounted(() => { hostElement.value = document.querySelector<HTMLElement>(".browser-host"); });
watch(() => grid.gridMode, () => nextTick(() => grid.forceGridRelayout()));
const gridLayouts = [
  { key: "grid", label: "▦ 宫格", title: "自动宫格平铺" },
  { key: "quad", label: "⊞ 四分", title: "四分 2×2" },
  { key: "horizontal", label: "▭ 横向", title: "横向一排" },
] as const;
const gridCounts = [2, 3, 4, 6, 9];
const urlsOpen = ref(false);
const resOpen = ref(false);
const resStats = ref<ResourceStats | null>(null);
let resTimer: number | null = null;

function setGridLayout(mode: (typeof gridLayouts)[number]["key"]) {
  const countChanged = mode === "quad" && grid.gridCount !== 4;
  grid.gridLayout = mode;
  if (mode === "quad") grid.gridCount = 4;
  if (!grid.gridOpen) return;
  if (countChanged) void grid.rebuildGrid(); else grid.layoutGrid();
}
function setGridCount(n: number) {
  if (grid.gridCount === n) return;
  grid.gridCount = n;
  if (n !== 4 && grid.gridLayout === "quad") grid.gridLayout = "grid";
  if (grid.gridOpen) void grid.rebuildGrid();
}
async function resRefresh() { try { resStats.value = await bridge.resourceStats(); } catch {} }
function toggleRes() {
  resOpen.value = !resOpen.value;
  if (resOpen.value) { void resRefresh(); resTimer = window.setInterval(resRefresh, 2000); }
  else if (resTimer) { clearInterval(resTimer); resTimer = null; }
}
function fmtMb(mb: number) { return mb >= 1024 ? (mb / 1024).toFixed(1) + "G" : Math.round(mb) + "M"; }
onBeforeUnmount(() => { if (resTimer) clearInterval(resTimer); });
</script>
<template>
  <div v-if="grid.gridOpen && layout.mainView === 'grid' && grid.gridMode === 'ai'" class="expand-row ai-send-row">
    <span class="er-label">🤖 群发</span>
    <input v-model="grid.gridAiInput" class="ai-send-input" placeholder="输入问题，同时发送给所有宫格中的 AI..." @keyup.enter="grid.gridSendAi" />
    <button class="er-primary" @click="grid.gridSendAi">发送</button>
  </div>
  <GridArchiveBar v-if="grid.gridOpen && layout.mainView === 'grid'" />
  <div v-if="layout.navSection === 'grid'" id="nav-grid-row" class="expand-row">
    <span class="er-label">模式</span>
    <button :class="{active:grid.gridMode==='browse'}" @click="grid.gridMode='browse'">🌐 浏览</button>
    <button :class="{active:grid.gridMode==='ai'}" @click="grid.gridMode='ai'">🤖 AI</button><span class="er-sep"/>
    <span class="er-label">布局</span>
    <button v-for="l in gridLayouts" :key="l.key" :class="{active:grid.gridLayout===l.key}" :title="l.title" @click="setGridLayout(l.key)">{{l.label}}</button><span class="er-sep"/>
    <span class="er-label">格数</span>
    <button v-for="n in gridCounts" :key="n" :class="{active:grid.gridCount===n}" @click="setGridCount(n)">{{n}}</button><span class="er-sep"/>
    <button class="er-primary" @click="grid.gridOpen ? grid.layoutGrid() : grid.activateGrid()">{{grid.gridOpen?'重排':'打开'}}</button>
    <button :class="{active:urlsOpen}" @click="urlsOpen=!urlsOpen">网址</button>
    <button :class="{active:resOpen}" @click="toggleRes">资源</button>
    <button class="er-danger" @click="layout.navSection=''; grid.closeGrid()">关闭宫格</button>
    <button class="er-close" @click="layout.navSection=''">✕</button>
  </div>
  <div v-if="layout.navSection==='grid' && resOpen && resStats" class="expand-row">
    <span class="er-label">系统可用 {{fmtMb(resStats.mem_available_mb)}} / {{fmtMb(resStats.mem_total_mb)}}</span><span class="er-sep"/>
    <span class="er-label">应用共 {{fmtMb(resStats.app_total_mb)}}</span><span class="er-label">主进程 {{fmtMb(resStats.main.rss_mb)}}</span>
    <span v-for="g in resStats.grids" :key="g.pid" class="er-label">{{g.name}} {{fmtMb(g.rss_mb)}}</span>
    <span v-if="resStats.mem_available_mb < 1500" class="er-warn">⚠️ 可用内存偏低，建议减少格数或关闭其它应用</span>
  </div>
  <div v-if="layout.navSection==='grid' && urlsOpen" class="expand-row">
    <span v-for="i in grid.gridCount" :key="i" class="er-url"><span class="er-gidx">{{i}}</span>
      <input v-model="grid.gridUrls[i-1]" :aria-label="'第 '+i+' 格网址'" placeholder="网址" @keyup.enter="grid.gridSetUrl(i-1)" />
      <button @click="grid.gridSetUrl(i-1)">↺</button></span>
  </div>
</template>
<style scoped>
.expand-row{display:flex;align-items:center;gap:5px;flex-wrap:wrap;padding:4px 8px;background:#f4f6fb;border-bottom:1px solid #e3e7f5}
.expand-row button{border:1px solid #d5dbe7;background:#fff;color:#4e5969;border-radius:5px;padding:3px 9px;font-size:11px;cursor:pointer;white-space:nowrap}
.expand-row button.active{background:#2b6cb0;border-color:#2b6cb0;color:#fff}.er-label{font-size:11px;color:#86909c}.er-warn{font-size:11px;color:#c0392b}.er-sep{width:1px;height:16px;background:#d5dbe7}
.er-primary{background:#2b6cb0!important;border-color:#2b6cb0!important;color:#fff!important}.er-danger{color:#c33!important}.er-close{margin-left:auto;border:none!important;background:transparent!important;color:#999!important}
.er-url{display:inline-flex;align-items:center;gap:3px}.er-gidx{font-size:10px;color:#86909c}.er-url input{width:170px;height:22px;border:1px solid #d5dbe7;border-radius:4px;font-size:11px;padding:0 6px}
</style>
