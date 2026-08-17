<script setup lang="ts">
import { ref } from "vue";
import { useBrowserStore } from "../../stores/useBrowserStore";
import { useLayoutStore } from "../../stores/useLayoutStore";
const browser = useBrowserStore();
const layout = useLayoutStore();

// 精简后只保留三种好用的布局
const layouts = [
  { key: "grid", label: "▦ 宫格", title: "自动宫格平铺" },
  { key: "quad", label: "⊞ 四分", title: "四分 2×2" },
  { key: "horizontal", label: "▭ 横向", title: "横向一排" },
] as const;

// 常用格数（不再 1~12 全列，太占地方）
const counts = [2, 3, 4, 6, 9];

const urlsOpen = ref(false); // 网址编辑区默认折叠

function setLayout(mode: (typeof layouts)[number]["key"]) {
  browser.gridLayout = mode;
  if (mode === "quad") browser.gridCount = 4;
  if (browser.gridOpen) browser.layoutGrid();
}
</script>

<template>
  <div v-if="layout.gridToolbarOpen" class="grid-toolbar">
    <div class="grid-toolbar-row">
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
          v-for="n in counts"
          :key="n"
          :class="{ active: browser.gridCount === n }"
          @click="browser.gridCount = n; browser.gridOpen && browser.layoutGrid()"
        >{{ n }}</button>
      </div>
      <button class="primary" @click="browser.gridOpen ? browser.layoutGrid() : browser.buildGrid()">
        {{ browser.gridOpen ? "重排" : "打开" }}
      </button>
      <button @click="urlsOpen = !urlsOpen" :class="{ active: urlsOpen }" title="编辑各格网址">网址</button>
      <button class="danger" @click="browser.closeGridAll">关闭</button>
      <button class="close" @click="layout.gridToolbarOpen = false" title="收起工具条">✕</button>
    </div>
    <div v-if="urlsOpen" class="grid-urls-inline">
      <div v-for="i in browser.gridCount" :key="i" class="grid-url-row">
        <span class="gidx">{{ i }}</span>
        <input v-model="browser.gridUrls[i - 1]" placeholder="网址" @keyup.enter="browser.gridSetUrl(i - 1)" />
        <button @click="browser.gridSetUrl(i - 1)">↺</button>
      </div>
    </div>
  </div>
</template>
