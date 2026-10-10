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
function bindHostAndRelayout() {
  nextTick(() => {
    hostElement.value = document.querySelector<HTMLElement>(".browser-host");
    if (hostElement.value && grid.gridOpen) grid.forceGridRelayout();
  });
}
onMounted(bindHostAndRelayout);
watch(() => layout.mainView, (view) => { if (view === "grid") bindHostAndRelayout(); });
watch(() => grid.gridMode, () => nextTick(() => grid.forceGridRelayout()));
const gridLayouts = [
  { key: "grid", label: "▦ 宫格", title: "自动宫格平铺" },
  { key: "quad", label: "⊞ 四分", title: "四分 2×2" },
  { key: "horizontal", label: "▭ 横向", title: "横向一排" },
] as const;
const gridCounts = [2, 3, 4, 6, 9];
const advancedOpen = ref(false);
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
function stopResourcePolling() {
  if (resTimer !== null) clearInterval(resTimer);
  resTimer = null;
}
function toggleRes() {
  resOpen.value = !resOpen.value;
  stopResourcePolling();
  if (resOpen.value && layout.mainView === "grid" && advancedOpen.value) {
    void resRefresh();
    resTimer = window.setInterval(resRefresh, 2000);
  }
}
// A hidden toolbar must not keep polling the native process tree.
watch(() => [layout.mainView, advancedOpen.value] as const, ([view]) => {
  if (view !== "grid" || !advancedOpen.value) stopResourcePolling();
  else if (resOpen.value && resTimer === null) {
    void resRefresh();
    resTimer = window.setInterval(resRefresh, 2000);
  }
});
function fmtMb(mb: number) { return mb >= 1024 ? (mb / 1024).toFixed(1) + "G" : Math.round(mb) + "M"; }
onBeforeUnmount(stopResourcePolling);
</script>
<template>
  <div v-if="grid.gridOpen && layout.mainView === 'grid' && grid.gridMode === 'ai'" class="expand-row ai-send-row">
    <span class="er-label">🤖 群发</span>
    <input v-model="grid.gridAiInput" class="ai-send-input" placeholder="输入问题，同时发送给所有宫格中的 AI..." @keyup.enter="grid.gridSendAi" />
    <button class="er-primary" @click="grid.gridSendAi">发送</button>
  </div>
  <GridArchiveBar v-if="grid.gridOpen && layout.mainView === 'grid'" />
  <div v-if="layout.mainView === 'grid'" class="grid-context" aria-label="宫格控制">
    <strong>对比工作区</strong>
    <div role="group" aria-label="模式"><button :class="{active:grid.gridMode==='browse'}" @click="grid.gridMode='browse'">浏览</button><button :class="{active:grid.gridMode==='ai'}" @click="grid.gridMode='ai'">AI</button></div>
    <div role="group" aria-label="快捷布局"><button v-for="n in [2,4]" :key="n" :class="{active:grid.gridCount===n}" @click="setGridCount(n)">{{n}} 格</button></div>
    <button class="grid-more" :aria-expanded="advancedOpen" aria-controls="grid-advanced" @click="advancedOpen=!advancedOpen">更多设置</button>
  </div>
  <div v-if="layout.mainView === 'grid' && advancedOpen" id="grid-advanced" class="grid-advanced">
    <label>布局 <select aria-label="宫格布局" :value="grid.gridLayout" @change="setGridLayout(($event.target as HTMLSelectElement).value as (typeof gridLayouts)[number]['key'])"><option v-for="l in gridLayouts" :key="l.key" :value="l.key">{{l.title}}</option></select></label>
    <label>格数 <select aria-label="格数" :value="grid.gridCount" @change="setGridCount(Number(($event.target as HTMLSelectElement).value))"><option v-for="n in gridCounts" :key="n" :value="n">{{n}}</option></select></label>
    <button @click="grid.gridOpen ? grid.layoutGrid() : grid.activateGrid()">重排</button><button @click="urlsOpen=!urlsOpen">编辑网址</button>
    <button @click="toggleRes">资源详情</button><button class="grid-danger" @click="grid.closeGrid();advancedOpen=false">关闭宫格</button>
  </div>
  <div v-if="layout.mainView==='grid' && advancedOpen && resOpen && resStats" class="expand-row">
    <span class="er-label">系统可用 {{fmtMb(resStats.mem_available_mb)}} / {{fmtMb(resStats.mem_total_mb)}}</span><span class="er-sep"/>
    <span class="er-label">应用共 {{fmtMb(resStats.app_total_mb)}}</span><span class="er-label">主进程 {{fmtMb(resStats.main.rss_mb)}}</span>
    <span v-for="g in resStats.grids" :key="g.pid" class="er-label">{{g.name}} {{fmtMb(g.rss_mb)}}</span>
    <span v-if="resStats.mem_available_mb < 1500" class="er-warn">⚠️ 可用内存偏低，建议减少格数或关闭其它应用</span>
  </div>
  <div v-if="layout.mainView==='grid' && advancedOpen && urlsOpen" class="expand-row">
    <span v-for="i in grid.gridCount" :key="i" class="er-url"><span class="er-gidx">{{i}}</span>
      <input v-model="grid.gridUrls[i-1]" :aria-label="'第 '+i+' 格网址'" placeholder="网址" @keyup.enter="grid.gridSetUrl(i-1)" />
      <button @click="grid.gridSetUrl(i-1)">↺</button></span>
  </div>
</template>
<style scoped>
.expand-row{display:flex;align-items:center;gap:5px;flex-wrap:wrap;padding:4px 8px;background:#f4f6fb;border-bottom:1px solid #e3e7f5}
.expand-row button{border:1px solid #d5dbe7;background:#fff;color:#4e5969;border-radius:5px;padding:3px 9px;font-size:11px;cursor:pointer;white-space:nowrap}
.er-label{font-size:11px;color:#86909c}.er-warn{font-size:11px;color:#c0392b}.er-sep{width:1px;height:16px;background:#d5dbe7}
.er-primary{background:#2b6cb0!important;border-color:#2b6cb0!important;color:#fff!important}
.er-url{display:inline-flex;align-items:center;gap:3px}.er-gidx{font-size:10px;color:#86909c}.er-url input{width:170px;height:22px;border:1px solid #d5dbe7;border-radius:4px;font-size:11px;padding:0 6px}
.grid-context,.grid-advanced { display:flex; flex-wrap:wrap; align-items:center; gap:10px; min-height:34px; padding:4px 12px; background:#f8fafc; border-bottom:1px solid #e2e8f0; font-size:12px; }
.grid-context [role=group] { display:inline-flex; border:1px solid #dbe4ef; border-radius:8px; overflow:hidden; }
.grid-context [role=group] button { border:0; border-radius:0; background:white; padding:5px 11px; }
.grid-context [role=group] button+button { border-left:1px solid #dbe4ef; }
.grid-context [role=group] button.active { background:#e7f1ff; color:#1e5c99; }
.grid-more { margin-left:auto; border:0!important; color:#245c99!important; background:transparent!important; }
.grid-advanced { background:#fff; }.grid-advanced label { display:flex; align-items:center; gap:6px; }.grid-advanced select { border:1px solid #dbe4ef; padding:4px; border-radius:6px; }.grid-danger { margin-left:auto; color:#b42318!important; }
</style>
