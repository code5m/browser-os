<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useBookmarkStore } from "../state/useBookmarkStore";
import { useBrowserStore } from "../../browser/public";
import { useFileStore } from "../../workspace/public";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { addUrlHomeFavorite, isHomeFavorite, removeHomeFavorite, toggleHomeDirectoryFavorite } from "../../home/public";
import { isLocalPath, isWebsite } from "../../../utils/omniboxSuggestions";

const bookmarks = useBookmarkStore();
const browser = useBrowserStore();
const fs = useFileStore();
const layout = useLayoutStore();
const pending = ref(false);
const target = computed(() => {
  if (layout.mainView === "files") return {kind: "dir" as const, value: fs.filePath};
  const raw = browser.url?.trim() || "";
  if (isLocalPath(raw)) return {kind: "dir" as const, value: raw};
  return {kind: "url" as const, value: isWebsite(raw) ? raw : (browser.activeTab?.url || "")};
});
const canFavorite = computed(() => target.value.kind === "dir"
  ? isLocalPath(target.value.value) : isWebsite(target.value.value));
const selected = computed(() => target.value.kind === "dir"
  ? isHomeFavorite("dir", target.value.value)
  : bookmarks.isBookmarked(target.value.value));
onMounted(() => { if (!bookmarks.loaded) void bookmarks.load(); });

async function toggleFavorite() {
  if (pending.value || !canFavorite.value) return;
  pending.value = true;
  try {
    if (target.value.kind === "dir") {
      toggleHomeDirectoryFavorite(target.value.value);
      return;
    }
    if (!bookmarks.loaded) await bookmarks.load();
    const url = target.value.value;
    const title = browser.activeTab?.title || url;
    const existing = bookmarks.findByUrl(url);
    if (existing) {
      if (await bookmarks.remove(existing.id)) {
        removeHomeFavorite("url", url);
        layout.showToast("已从收藏夹及主页移除");
      }
      return;
    }
    const wasHome = isHomeFavorite("url", url);
    if (!addUrlHomeFavorite(url, title)) return; // at capacity: don't create half-favorite
    if (!(await bookmarks.add(url, title))) {
      if (!wasHome) removeHomeFavorite("url", url); // compensate native failure
      return;
    }
    layout.showToast("已同时收藏到收藏夹和主页");
  } finally {
    pending.value = false;
  }
}
function openPanel() {
  if (layout.mainView !== "browser") layout.activateBrowser();
  bookmarks.togglePanel();
}
</script>

<template>
  <span class="address-favorites" role="group" aria-label="快捷收藏与收藏夹">
    <button type="button" class="fav-action" :class="{ selected }" :disabled="!canFavorite || pending"
      :aria-pressed="selected" :aria-label="target.kind === 'dir' ? '收藏或取消收藏当前目录到主页' : '收藏或取消收藏网址到收藏夹和主页'"
      :title="target.kind === 'dir' ? '目录：收藏到主页' : '网址：收藏夹与主页同步收藏'"
      @mousedown.prevent @click="toggleFavorite">{{ selected ? "★" : "☆" }}</button>
    <button type="button" class="fav-panel" aria-label="收藏夹" title="打开收藏夹"
      :aria-expanded="bookmarks.panelOpen" @mousedown.prevent @click="openPanel">▤</button>
  </span>
</template>
<style scoped>
.address-favorites{display:inline-flex;align-items:center;gap:2px;flex:none}
.address-favorites button{border:0;background:transparent;min-width:25px;height:24px;border-radius:5px;cursor:pointer;color:#52677d;font-size:15px}
.address-favorites button:hover:not(:disabled){background:#e4edf8;color:#2563a4}
.address-favorites button.selected{color:#1d65b1}
.address-favorites button:disabled{opacity:.4;cursor:default}
</style>
