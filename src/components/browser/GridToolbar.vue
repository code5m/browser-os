<script setup lang="ts">
import { useBrowserStore } from "../../stores/useBrowserStore";
import { useLayoutStore } from "../../stores/useLayoutStore";
const browser = useBrowserStore();
const layout = useLayoutStore();
</script>

<template>
  <div v-if="layout.gridToolbarOpen" class="grid-toolbar">
    <div class="grid-toolbar-row">
      <span class="gt-label">▦ 宫格对比</span>
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
