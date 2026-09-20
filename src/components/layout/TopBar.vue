<script setup lang="ts">
import { useBrowserStore } from "../../capabilities/browser/public";
import { useLayoutStore } from "../../stores/useLayoutStore";

const browser = useBrowserStore();
const layout = useLayoutStore();
</script>

<template>
  <header class="topbar">
    <button @click="browser.goBack" title="后退">←</button>
    <button @click="browser.goForward" title="前进">→</button>
    <button @click="browser.reloadActive" title="刷新">↻</button>
    <input
      v-model="browser.url"
      placeholder="输入网址或搜索词（如 baidu.com、天气）"
      @keyup.enter="browser.openBrowser"
    />
    <button class="primary" @click="browser.openBrowser">打开浏览</button>
    <button @click="browser.tabNew()" title="新建页签">＋ 页签</button>
    <button v-if="layout.mainView === 'editor'" @click="layout.setView('browser'); browser.relocate()" title="返回浏览器">🌐 浏览器</button>
    <span class="msg">{{ layout.msg }}</span>
  </header>
</template>
