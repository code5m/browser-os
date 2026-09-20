<script setup lang="ts">
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { useBookmarkStore } from "../state/useBookmarkStore";

// 顶部工具栏尾部的「收藏夹」入口按钮（M1-3）。
// 作为 Bookmark 能力的 navigation contribution（slot=activity-bar-trailing）挂到 Shell，
// 使 Shell 不持有该按钮的 Bookmark 专属知识。
// 行为与原 ActivityBar 内联按钮一致：任意视图点击都应"打开"收藏夹面板，而非简单取反。
const layout = useLayoutStore();
const bookmarks = useBookmarkStore();

function onToggleBookmarkPanel(): void {
  if (layout.mainView !== "browser") {
    layout.activateBrowser();
    if (!bookmarks.panelOpen) bookmarks.togglePanel();
    return;
  }
  bookmarks.togglePanel();
}
</script>

<template>
  <button
    class="tbtn bookmark-entry"
    :class="{ active: bookmarks.panelOpen }"
    aria-label="收藏夹"
    title="打开收藏夹"
    @click="onToggleBookmarkPanel"
  >📑 <span>收藏夹</span></button>
</template>

<style scoped>
.bookmark-entry {
  color: #cbd5e0;
}
.bookmark-entry:hover {
  background: #2f3a47;
  color: #fff;
}
.bookmark-entry.active {
  background: #0e639c;
  color: #fff;
}
</style>
