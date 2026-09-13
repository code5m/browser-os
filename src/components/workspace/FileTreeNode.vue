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

// ===== 拖拽移动 =====
function onDragStart(ev: DragEvent) {
  ws.startDrag(props.entry.path, ev);
}
function onDragEnd() {
  ws.dragSource = "";
}
function onDragOver(ev: DragEvent) {
  if (!props.entry.is_dir) return; // 只有目录可作为放置目标
  ev.preventDefault();
  ev.stopPropagation(); // 避免冒泡到父目录误高亮
  ws.onDirDragOver(props.entry.path, ev);
}
function onDragLeave() {
  if (!props.entry.is_dir) return;
  ws.onDirDragLeave(props.entry.path);
}
function onDrop(ev: DragEvent) {
  if (!props.entry.is_dir) return;
  ev.preventDefault();
  ev.stopPropagation();
  ws.onDirDrop(props.entry.path, ev);
}
</script>

<template>
  <div
    class="tnode"
    :class="{ active: ws.inlineFile === entry.path, locate: ws.locateTarget === entry.path, drop: ws.dropTarget === entry.path }"
    :style="{ paddingLeft: indent }"
    :title="entry.path"
    :data-path="entry.path"
    draggable="true"
    @click="onClick"
    @contextmenu="ws.onFileContext($event, entry)"
    @dragstart="onDragStart"
    @dragend="onDragEnd"
    @dragover="onDragOver"
    @dragleave="onDragLeave"
    @drop="onDrop"
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

<style scoped>
.tnode {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 6px;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
  user-select: none;
}
.tnode:hover {
  background: #eef1f6;
}
.tnode.active {
  background: #dbeafe;
  color: #1d4ed8;
  font-weight: 600;
}
.tnode.locate {
  background: #fff3cd;
  box-shadow: inset 3px 0 0 #f0a500;
}
.tnode.drop {
  background: #d6f5d6;
  outline: 1px dashed #2e9e2e;
}
</style>
