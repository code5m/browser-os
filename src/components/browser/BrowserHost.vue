<script setup lang="ts">
import { useBrowserStore } from "../../stores/useBrowserStore";
import { useBrowserHost } from "../../composables/useBrowserHost";

const browser = useBrowserStore();
const { browserHost } = useBrowserHost();
</script>

<template>
  <!-- 始终挂载，用 CSS 隐藏而非 v-show(display:none)，保证 getBoundingClientRect 始终
       拿到真实非零尺寸，避免子 webview 定位矩形算错、网页跑出 data-v-app 容器 -->
  <div
    ref="browserHost"
    class="browser-host"
    :style="{ visibility: browser.isBrowserVisible ? 'visible' : 'hidden' }"
  >
    <div v-if="!browser.tabs.length" class="host-hint">
      点「打开浏览」或「＋ 页签」在此内嵌网页（可多页签并排切换）
    </div>
  </div>
</template>
