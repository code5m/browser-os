<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { Search, X } from '@lucide/vue';
import { MODULE_META } from '../../../stores/useLayoutStore';
import { useWorkbenchStore } from '../state/useWorkbenchStore';
const workbench = useWorkbenchStore();
const input = ref<HTMLInputElement>();
const query = ref('');
const selected = ref(0);
let previous: HTMLElement | null = null;
const commands = computed(() => [
  { id: 'browser', label: '浏览网页', run: () => workbench.open('browser') },
  ...Object.entries(MODULE_META).map(([id, value]) => ({ id, label: value.label, run: () => workbench.open(id) })),
  { id: 'collapse', label: workbench.collapsed ? '恢复工具窗' : '折叠全部工具窗', run: () => workbench.toggleTools() },
].filter(c => `${c.id} ${c.label}`.toLowerCase().includes(query.value.toLowerCase())).slice(0, 24));
watch(query, () => selected.value = 0);
watch(() => workbench.commandOpen, async open => {
  if (open) { previous = document.activeElement as HTMLElement; query.value = ''; selected.value = 0; await nextTick(); input.value?.focus(); }
  else previous?.focus();
});
function run(index: number) { commands.value[index]?.run(); workbench.commandOpen = false; }
function keys(e: KeyboardEvent) {
  if (e.isComposing) return;
  if ((e.ctrlKey || e.metaKey) && ((e.shiftKey && e.key.toLowerCase() === 'p') || (!e.shiftKey && e.key.toLowerCase() === 'k'))) {
    e.preventDefault(); workbench.commandOpen = !workbench.commandOpen;
  } else if (workbench.commandOpen && e.key === 'Escape') { e.preventDefault(); workbench.commandOpen = false; }
}
function navigate(e: KeyboardEvent) {
  if (e.isComposing) return;
  if (e.key === 'Enter') { e.preventDefault(); run(selected.value); }
  if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
    e.preventDefault(); selected.value = (selected.value + (e.key === 'ArrowDown' ? 1 : -1) + commands.value.length) % (commands.value.length || 1);
    document.getElementById(`command-${selected.value}`)?.scrollIntoView({ block: 'nearest' });
  }
}
onMounted(() => window.addEventListener('keydown', keys));
onUnmounted(() => window.removeEventListener('keydown', keys));
</script>
<template>
  <section v-if="workbench.commandOpen" class="command-center" aria-label="统一命令入口">
    <div class="command-search"><Search :size="16"/><input ref="input" v-model="query" role="combobox" aria-label="搜索命令" aria-controls="command-results" :aria-activedescendant="`command-${selected}`" aria-expanded="true" @keydown="navigate"/><button title="关闭命令入口" aria-label="关闭命令入口" @click="workbench.commandOpen = false"><X :size="16"/></button></div>
    <div id="command-results" role="listbox" class="command-results"><button v-for="(cmd,i) in commands" :id="`command-${i}`" :key="cmd.id" role="option" :aria-selected="selected === i" @click="run(i)">{{ cmd.label }}<small>{{ cmd.id }}</small></button><span v-if="!commands.length">没有匹配的命令</span></div>
  </section>
</template>
<style scoped>
.command-center{flex:none;background:#fff;border-bottom:1px solid #d6dce2;padding:6px 12px;color:#25313c}
.command-search{display:flex;align-items:center;gap:8px}.command-search input{flex:1;min-width:0;padding:7px;border:1px solid #bbc5cf;border-radius:4px}
.command-search button{width:28px;height:28px;display:grid;place-items:center}
.command-results{display:flex;flex-direction:column;max-height:180px;overflow:auto;padding-top:4px}
.command-results button{flex:none;display:flex;justify-content:space-between;gap:8px;border:0;border-radius:3px;background:white;padding:6px 10px;text-align:left;color:inherit}
.command-results button[aria-selected=true]{background:#e3f1ea}.command-results small{color:#637581}
</style>
