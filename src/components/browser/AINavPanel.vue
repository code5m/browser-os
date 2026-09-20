<script setup lang="ts">
import { computed } from "vue";
import { useBrowserStore } from "../../capabilities/browser/public";

const browser = useBrowserStore();
const aiFiltered = computed(() => browser.aiFiltered);
</script>

<template>
  <aside v-if="browser.aiNavOpen" class="ai-nav">
    <div class="ai-head">
      <span>🤖 AI 导航</span>
      <div class="ai-filters">
        <button :class="{ active: browser.aiFilter === '全部' }" @click="browser.aiFilter = '全部'">全部</button>
        <button :class="{ active: browser.aiFilter === '国内' }" @click="browser.aiFilter = '国内'">国内</button>
        <button :class="{ active: browser.aiFilter === '海外' }" @click="browser.aiFilter = '海外'">海外</button>
      </div>
      <button class="close" @click="browser.aiNavOpen = false" title="收起">✕</button>
    </div>
    <ul class="ai-list">
      <li v-for="s in aiFiltered" :key="s.url" class="ai-item" @click="browser.gotoAI(s)">
        <span class="ai-name">{{ s.name }}</span>
        <span class="ai-region" :class="s.region === '国内' ? 'cn' : 'ov'">{{ s.region }}</span>
      </li>
    </ul>
    <div class="ai-tip">点击站点 → 在内嵌浏览器打开</div>
  </aside>
</template>
