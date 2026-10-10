<script setup lang="ts">
import { useLayoutStore, isNavActive } from "../../../stores/useLayoutStore";
import { useGridStore } from "../public";
const layout = useLayoutStore();
const grid = useGridStore();
function activate() {
  layout.navSection = "";
  grid.gridMode = "ai";
  void grid.activateGrid().catch(() => layout.showToast("打开宫格失败，请重试"));
}
</script>
<template>
  <button data-nav-item :class="{ active: isNavActive(layout.mainView, 'grid') }" aria-label="打开宫格" title="打开宫格" @click="activate">
    <span class="ic">🗂️</span><span class="lab">宫格</span>
  </button>
  <button class="caret-btn" data-nav-item data-nav-toggle="grid" :class="{ active: layout.navSection === 'grid' }"
    aria-label="宫格设置" :aria-expanded="layout.navSection === 'grid'" aria-controls="nav-grid-row" title="宫格设置"
    @click.stop="layout.toggleNavSection('grid')">▾</button>
</template>
<style scoped>
.caret-btn{min-width:16px;padding:2px 3px 2px 0;font-size:9px;color:#7d8794}.caret-btn.active{color:#fff}.lab{display:none}
</style>
