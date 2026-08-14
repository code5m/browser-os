<script setup lang="ts">
import { useBrowserStore } from "../../stores/useBrowserStore";
const browser = useBrowserStore();
</script>

<template>
  <div v-if="browser.isBrowserVisible && browser.tabs.length" class="tabbar">
    <div
      v-for="t in browser.tabs"
      :key="t.id"
      :class="['tab', { active: t.id === browser.activeTabId }]"
      @click="browser.tabSwitch(t.id)"
      :title="t.url"
    >
      <span class="tab-title">{{ t.title || t.url }}</span>
      <button class="tab-close" @click.stop="browser.tabClose(t.id)" title="关闭">✕</button>
    </div>
    <button class="tab-new" @click="browser.tabNew()" title="新建页签">＋</button>
  </div>
</template>
