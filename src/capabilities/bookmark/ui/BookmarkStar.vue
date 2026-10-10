<script setup lang="ts">
import { onMounted } from "vue";
import { useBookmarkStore } from "../state/useBookmarkStore";
import { useLayoutStore } from "../../../stores/useLayoutStore";

const bookmarks = useBookmarkStore();
const layout = useLayoutStore();

// UI event adapter: navigation is owned by Layout, panel visibility by the
// existing canonical useBookmarkStore.togglePanel intent. No second intent/state.
function onBookmarkClick() {
  if (layout.mainView !== "browser") {
    layout.activateBrowser();
    if (!bookmarks.panelOpen) bookmarks.togglePanel();
    return;
  }
  bookmarks.togglePanel();
}

onMounted(() => {
  if (!bookmarks.loaded) bookmarks.load();
});
</script>

<template>
  <button class="star-btn panel-btn" :class="{ active: bookmarks.panelOpen }"
    aria-label="收藏夹" :aria-expanded="bookmarks.panelOpen" title="收藏夹"
    @click="onBookmarkClick">☆</button>
</template>

<style scoped>
.star-btn {
  flex-shrink: 0;
  min-width: 22px;
  height: 24px;
  padding: 0 3px;
  border: none;
  background: transparent;
  color: #86909c;
  border-radius: 5px;
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
.star-btn:hover:not(:disabled) {
  background: #2f3a47;
}
.panel-btn {
  font-size: 13px;
}
.panel-btn.active {
  background: #2f3a47;
}
</style>
