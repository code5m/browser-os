<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useBrowserStore } from "../../browser/public";
import { useBookmarkStore, canBookmark } from "../state/useBookmarkStore";
import { useLayoutStore } from "../../../stores/useLayoutStore";

// 地址栏 ⭐ 收藏按钮（M1-3）：挂在智能地址栏右侧，消费 bridge.bookmarkAdd/Remove。
// 与 M1-0 的 useHomeStore「收藏当前网页到主页」是两套独立机制：
// 主页快捷方式存 localStorage，这里走后端 bookmarks.json，互不并入。
const browser = useBrowserStore();
const bookmarks = useBookmarkStore();
const layout = useLayoutStore();
function openBookmarkPanel() {
  if (layout.mainView !== "browser") {
    layout.activateBrowser();
    if (!bookmarks.panelOpen) bookmarks.togglePanel();
  } else bookmarks.togglePanel();
}

// 收藏判定用"激活页签 URL"，没有页签时退回地址栏输入值
const currentUrl = computed(() =>
  (browser.activeTab?.url || browser.url || "").trim()
);
const currentTitle = computed(() => browser.activeTab?.title || currentUrl.value);
const enabled = computed(() => canBookmark(currentUrl.value));
const starred = computed(() => bookmarks.isBookmarked(currentUrl.value));

// 首屏就要知道当前页是否已收藏：ActivityBar 常驻挂载，这里是最早的可靠加载点
onMounted(() => {
  if (!bookmarks.loaded) bookmarks.load();
});

async function onStar() {
  if (!enabled.value || bookmarks.busy) return;
  await bookmarks.toggle(currentUrl.value, currentTitle.value);
}
</script>

<template>
  <button class="star-btn panel-btn" :class="{ active: bookmarks.panelOpen }"
    aria-label="收藏夹" :aria-expanded="bookmarks.panelOpen" title="收藏夹"
    @click="openBookmarkPanel">☆</button>
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
/* 已收藏：高亮金色实心星，未收藏为空心星 */
.star-btn.starred {
  color: #f2b134;
}
.star-btn.disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.star-btn:disabled {
  cursor: not-allowed;
}
.panel-btn {
  font-size: 13px;
}
.panel-btn.active {
  background: #2f3a47;
}
</style>
