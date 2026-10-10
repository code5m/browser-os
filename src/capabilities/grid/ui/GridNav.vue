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
</template>
<style scoped>
.lab { display: none; }
</style>
