<script setup lang="ts">
import { computed, onMounted } from "vue";
import type { Bookmark } from "../../types";
import { useBrowserStore } from "../../stores/useBrowserStore";
import { useBookmarkStore } from "../../stores/useBookmarkStore";
import { useLayoutStore } from "../../stores/useLayoutStore";

// 收藏夹侧栏（M1-3）：按 created_at 倒序展示 bridge.bookmarkList()，
// 点项在内嵌浏览器打开（不走系统默认浏览器，那是 M1-4），按 id 删除。
const browser = useBrowserStore();
const bookmarks = useBookmarkStore();
const layout = useLayoutStore();

// 侧栏可能先于 ⭐ 按钮挂载（如刷新后直接展开），这里兜底加载一次
onMounted(() => {
  if (!bookmarks.loaded) bookmarks.load();
});

const list = computed(() => bookmarks.sorted);

function hostOf(u: string): string {
  try {
    return new URL(u).hostname;
  } catch {
    return u;
  }
}

// 侧栏点击一律开内嵌页签：openBrowser 走 bridge.tabNew，不会触发系统默认浏览器
async function openItem(b: Bookmark) {
  browser.url = b.url;
  layout.setView("browser");
  await browser.openBrowser();
}

async function removeItem(b: Bookmark) {
  if (bookmarks.busy) return;
  await bookmarks.remove(b.id);
}
</script>

<template>
  <aside class="bookmark-side">
    <div class="tabs">
      <span class="bm-title">📑 收藏夹</span>
      <span class="bm-count">{{ list.length }}</span>
      <button title="刷新" @click="bookmarks.load()">↻</button>
      <button class="close" title="收起" @click="bookmarks.togglePanel">✕</button>
    </div>
    <div v-if="bookmarks.error" class="bm-error">{{ bookmarks.error }}</div>
    <div class="bm-list">
      <template v-if="list.length">
        <div v-for="b in list" :key="b.id" class="bm-item">
          <button class="bm-open" :title="b.url" @click="openItem(b)">
            <span class="bm-name">{{ b.title || b.url }}</span>
            <span class="bm-host">{{ hostOf(b.url) }}</span>
          </button>
          <button class="bm-del" title="删除这条收藏" @click="removeItem(b)">✕</button>
        </div>
      </template>
      <div v-else class="bm-empty">
        {{ bookmarks.loaded ? "还没有收藏：点地址栏 ☆ 收藏当前网页" : "正在读取收藏夹…" }}
      </div>
    </div>
  </aside>
</template>

<style scoped>
.bookmark-side {
  width: 260px;
  flex-shrink: 0;
  border-right: 1px solid #e5e6eb;
  display: flex;
  flex-direction: column;
  background: #f7f8fa;
  min-height: 0;
}
.tabs {
  flex-shrink: 0;
}
.bm-title {
  font-size: 13px;
  font-weight: 600;
  color: #4e5969;
  padding: 0 4px 0 2px;
}
.bm-count {
  font-size: 11px;
  color: #86909c;
  margin-right: auto;
}
.tabs button {
  background: none;
  border: none;
  border-bottom: none;
  padding: 6px 8px;
  cursor: pointer;
  font-size: 13px;
  color: #86909c;
}
.tabs button:hover {
  color: #2b6cb0;
}
.tabs .close {
  color: #bbb;
}
.bm-error {
  flex-shrink: 0;
  font-size: 11px;
  color: #c0392b;
  padding: 4px 8px;
  background: #fdecea;
  border-bottom: 1px solid #f5c6c2;
  word-break: break-all;
}
.bm-list {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  padding: 4px;
}
.bm-item {
  display: flex;
  align-items: center;
  gap: 2px;
  border-radius: 5px;
}
.bm-item:hover {
  background: #eef2f7;
}
.bm-open {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 1px;
  background: none;
  border: none;
  padding: 5px 6px;
  cursor: pointer;
  text-align: left;
}
.bm-name {
  font-size: 12px;
  color: #1d2129;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.bm-host {
  font-size: 10px;
  color: #86909c;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.bm-del {
  flex-shrink: 0;
  background: none;
  border: none;
  color: #bbb;
  cursor: pointer;
  font-size: 12px;
  padding: 4px 6px;
  border-radius: 4px;
}
.bm-del:hover {
  color: #c0392b;
  background: #fdecea;
}
.bm-empty {
  font-size: 12px;
  color: #86909c;
  padding: 12px 8px;
  line-height: 1.6;
}
</style>
