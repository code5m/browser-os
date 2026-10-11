<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useBookmarkStore } from "../state/useBookmarkStore";
const props = defineProps<{ query: string; mode: "auto" | "url" | "dir" }>();
const emit = defineEmits<{ (e: "choose-url", url: string): void }>();
const store = useBookmarkStore();
const fileInput = ref<HTMLInputElement | null>(null);
const visible = computed(() => props.mode === "dir" ? [] : store.sorted.filter(b =>
  !props.query || (b.title + " " + b.url).toLowerCase().includes(props.query.toLowerCase())
).slice(0,5));
onMounted(() => { if (!store.loaded) void store.load(); });
async function importFile(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  try { await store.importFile(file); }
  catch { store.error = "无法导入收藏夹文件，请选择浏览器导出的 HTML 或 JSON"; }
  finally { input.value = ""; }
}
</script>
<template>
  <div v-if="props.mode !== 'dir'" class="omni-group">
    <div class="omni-heading">
      <strong>收藏夹</strong>
      <button type="button" @mousedown.prevent @click="fileInput?.click()">导入浏览器收藏栏</button>
      <input ref="fileInput" type="file" accept=".html,.htm,.json" class="omni-file" aria-label="选择浏览器书签导出文件" @change="importFile" />
    </div>
    <button v-for="b in visible" :key="b.id" type="button" role="option" class="omni-entry"
      @mousedown.prevent @click="emit('choose-url', b.url)">
      <span aria-hidden="true">★</span><span class="entry-name">{{ b.title || b.url }}</span><small>收藏网址</small>
    </button>
    <p v-if="store.error" class="omni-warning" role="status">收藏数据载入或导入失败，请检查文件格式。</p>
  </div>
</template>
<style scoped>
.omni-group{padding:3px 7px 5px;border-top:1px solid #ecf0f4}
.omni-heading{display:flex;align-items:center;justify-content:space-between;gap:10px;padding:4px;color:#54677f;font-size:11px}
.omni-heading button{border:0;background:none;color:#2667a7;cursor:pointer;font-size:11px}
.omni-file{display:none}
.omni-entry{display:flex;align-items:center;gap:8px;padding:5px 12px;width:100%;border:0;background:none;text-align:left;cursor:pointer;color:#34516f;font-size:12px}
.omni-entry:hover{background:#eef4fb}
.entry-name{min-width:0;flex:1;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.omni-entry small{color:#8593a4}
.omni-warning{color:#8a4e32;font-size:11px}
</style>
