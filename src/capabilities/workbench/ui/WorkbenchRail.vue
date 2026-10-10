<script setup lang="ts">
import { Folder, Database, Notebook, GitBranch, Terminal, PanelLeftOpen } from '@lucide/vue';
import { useWorkbenchStore } from '../state/useWorkbenchStore';
import { useLayoutStore } from '../../../stores/useLayoutStore';
const workbench = useWorkbenchStore();
const layout = useLayoutStore();
const tools = [{ id:'files', name:'文件', icon:Folder }, { id:'tools', name:'全部工具', icon:Database }];
</script>
<template><nav class="workbench-rail" aria-label="工具窗"><button v-if="workbench.collapsed" aria-label="恢复工具窗" title="恢复工具窗" @click="workbench.toggleTools()"><PanelLeftOpen :size="16"/></button><template v-else><button v-for="t in tools" :key="t.id" :title="t.name" :aria-label="t.name" :aria-pressed="layout.mainView === t.id" @click="workbench.open(t.id)"><component :is="t.icon" :size="16"/></button></template></nav></template>
<style scoped>.workbench-rail{width:28px;flex:0 0 28px;background:#f1f4f5;border-right:1px solid #d8dfe3;display:flex;flex-direction:column;padding-top:4px;gap:6px}.workbench-rail button{width:28px;height:30px;padding:0;display:grid;place-items:center;background:none;border:0;border-radius:0;color:#526571}.workbench-rail button[aria-pressed=true]{background:#dbede5;color:#12634b;border-left:2px solid #188565}</style>
