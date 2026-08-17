<script setup lang="ts">
import { useBrowserStore } from "../../stores/useBrowserStore";
import { useLayoutStore } from "../../stores/useLayoutStore";
const browser = useBrowserStore();
const layout = useLayoutStore();

const layouts = [
  { key: "horizontal", label: "▭ 横向", title: "横向一排" },
  { key: "vertical", label: "▯ 纵向", title: "纵向一列" },
  { key: "quad", label: "⊞ 四分", title: "四分 2×2" },
  { key: "grid", label: "▦ 宫格", title: "自动宫格" },
  { key: "free", label: "❖ 自由", title: "自由层叠" },
] as const;

function setLayout(mode: (typeof layouts)[number]["key"]) {
  browser.gridLayout = mode;
  // 四分模式固定 4 格，更符合语义
  if (mode === "quad") browser.gridCount = 4;
  // 已打开宫格则按新布局重排
  if (browser.gridOpen) browser.layoutGrid();
}
</script>

<template>
  <div v-if="layout.gridToolbarOpen" class="grid-toolbar">
    <div class="grid-toolbar-row">
      <span class="gt-label">▦ 宫格对比</span>
      <div class="layout-btns">
        <button
          v-for="l in layouts"
          :key="l.key"
          :class="{ active: browser.gridLayout === l.key }"
          :title="l.title"
          @click="setLayout(l.key)"
        >{{ l.label }}</button>
      </div>
      <div class="grid-btns">
        <button
          v-for="n in 11"
          :key="n"
          :class="{ active: browser.gridCount === n + 1 }"
          @click="browser.gridCount = n + 1"
        >{{ n + 1 }}</button>
      </div>
      <button class="primary" @click="browser.buildGrid">打开</button>
      <button @click="browser.gridOpen ? browser.layoutGrid() : browser.buildGrid()">重排</button>
      <button class="danger" @click="browser.closeGridAll">关闭</button>
      <button class="close" @click="layout.gridToolbarOpen = false" title="收起工具条">✕</button>
    </div>
    <div class="grid-urls-inline">
      <div v-for="i in browser.gridCount" :key="i" class="grid-url-row">
        <span class="gidx">{{ i }}</span>
        <input v-model="browser.gridUrls[i - 1]" placeholder="网址" @keyup.enter="browser.gridSetUrl(i - 1)" />
        <button @click="browser.gridSetUrl(i - 1)">↺</button>
      </div>
    </div>
  </div>
</template>
