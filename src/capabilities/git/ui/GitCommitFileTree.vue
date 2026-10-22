<script setup lang="ts">
type FileEntry = {path:string;status:string;previous_path?:string};
type TreeNode = {name:string;path:string;children:TreeNode[];files:FileEntry[]};
defineProps<{node:TreeNode;selectedPath:string}>();
const emit = defineEmits<{select:[path:string]}>();
</script>
<template>
  <button v-for="file in node.files" :key="file.path" :class="{active:selectedPath===file.path}" :aria-pressed="selectedPath===file.path" :title="file.previous_path ? `${file.previous_path} → ${file.path}` : file.path" @click="emit('select',file.path)"><small>{{file.status}}</small> {{file.path.split('/').at(-1)}}<small v-if="file.previous_path"> ← {{file.previous_path}}</small></button>
  <details v-for="child in node.children" :key="child.path" open class="commit-directory"><summary>{{child.name}}</summary><GitCommitFileTree :node="child" :selected-path="selectedPath" @select="emit('select',$event)"/></details>
</template>

<style scoped>
button {
  display: block;
  box-sizing: border-box;
  width: 100%;
  padding: 4px 6px;
  overflow: hidden;
  color: inherit;
  text-align: left;
  text-overflow: ellipsis;
  white-space: nowrap;
  background: transparent;
  border: 0;
  border-bottom: 1px solid var(--git-border-soft, #e6edf4);
  border-radius: 0;
}
button:hover,
button.active {
  color: #fff;
  background: var(--git-selected, #dcebf9);
}
button small:first-child {
  display: inline-block;
  min-width: 18px;
  font-weight: 700;
}
.commit-directory {
  margin-left: 8px;
}
.commit-directory > summary {
  padding: 3px 4px;
  overflow: hidden;
  color: inherit;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
  cursor: pointer;
}
</style>
