<script setup lang="ts">
import { computed } from "vue";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";

// 递归树节点：文件夹点击展开/收起（懒加载子级），文件点击打开
const props = defineProps<{ entry: DirEntry; depth: number; ide: boolean }>();
const ws = useWorkspaceStore();

const expanded = computed(() => ws.treeExpanded.has(props.entry.path));
const loading = computed(() => ws.treeLoading.has(props.entry.path));
const children = computed(() => ws.treeChildren.get(props.entry.path) || []);
const isMd = computed(() => /\.(md|markdown)$/i.test(props.entry.name));
const indent = computed(() => 8 + props.depth * 14 + "px");

function onClick() {
  if (props.entry.is_dir) ws.toggleTreeDir(props.entry.path);
  else if (props.ide) ws.openFileInline(props.entry);
  else ws.openFile(props.entry); // Dock 窄栏：走 overlay 编辑器
}
</script>

<template>
  <div
    class="tnode"
    :class="{ active: ws.inlineFile === entry.path }"
    :style="{ paddingLeft: indent }"
    :title="entry.path"
    @click="onClick"
    @contextmenu="ws.onFileContext($event, entry)"
  >
    <span class="twisty">{{ entry.is_dir ? (expanded ? "▾" : "▸") : "" }}</span>
    <span class="icon">{{ entry.is_dir ? "📁" : isMd ? "📝" : "📄" }}</span>
    <span class="fname">{{ entry.name }}</span>
  </div>
  <template v-if="entry.is_dir && expanded">
    <div v-if="loading" class="tnode" :style="{ paddingLeft: 8 + (depth + 1) * 14 + 'px' }">
      <span class="fname dim">加载中…</span>
    </div>
    <FileTreeNode
      v-for="c in children"
      :key="c.path"
      :entry="c"
      :depth="depth + 1"
      :ide="ide"
    />
    <div
      v-if="!loading && !children.length"
      class="tnode"
      :style="{ paddingLeft: 8 + (depth + 1) * 14 + 'px' }"
    >
      <span class="fname dim">(空目录)</span>
    </div>
  </template>
</template>
