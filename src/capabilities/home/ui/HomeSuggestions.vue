<script setup lang="ts">
import { computed } from "vue";
import { useHomeStore } from "../state/useHomeStore";
const props = defineProps<{ query: string; mode: "auto" | "url" | "dir" }>();
const emit = defineEmits<{
  (e: "choose-url", url: string): void;
  (e: "choose-dir", path: string): void;
}>();
const home = useHomeStore();
const visible = computed(() => home.shortcuts.filter(s =>
  (s.type === "url" && props.mode !== "dir" || s.type === "dir" && props.mode !== "url") &&
  (!props.query || (s.name + " " + s.target).toLowerCase().includes(props.query.toLowerCase()))
).slice(0,5));
function choose(s: (typeof home.shortcuts)[number]) {
  if (s.type === "dir") emit("choose-dir", s.target);
  else emit("choose-url", s.target);
}
</script>
<template>
  <div v-if="visible.length" class="omni-group">
    <strong class="omni-heading">主页收藏</strong>
    <button v-for="s in visible" :key="s.id" role="option" type="button" class="omni-entry"
      @mousedown.prevent @click="choose(s)">
      <span aria-hidden="true">{{ s.type === "dir" ? "📁" : "🌐" }}</span>
      <span class="entry-name">{{ s.name }}</span>
      <small>{{ s.type === "dir" ? "目录" : "网址" }}</small>
    </button>
  </div>
</template>
<style scoped>
.omni-group{border-top:1px solid #ecf0f4;padding:5px 7px}
.omni-heading{display:block;color:#54677f;font-size:11px;margin:3px 12px}
.omni-entry{width:100%;display:flex;align-items:center;gap:8px;border:0;background:none;padding:6px 12px;text-align:left;color:#35465b;cursor:pointer;font-size:12px}
.omni-entry:hover{background:#eef4fb}
.entry-name{flex:1;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.omni-entry small{color:#8393a7}
</style>
