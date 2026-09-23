<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { Marked } from 'marked';
import DOMPurify from 'dompurify';
import { FolderOpen, RefreshCw, Network, PanelLeftClose, PanelLeftOpen } from '@lucide/vue';
import { useVaultStore } from '../state/useVaultStore';
import { useWorkbenchStore } from '../../../stores/useWorkbenchStore';
import { layoutPositions } from '../../../utils/graphUi';
const vault = useVaultStore();
const workbench = useWorkbenchStore();
const treeOpen = ref(true);
const graphOpen = ref(true);
const reader = ref<HTMLElement>();
const collapsed = ref(new Set<string>());
const folders = computed(() => [...new Set(vault.vaultResults.map(n => n.path.split('/').slice(0,-1).join('/') || '/'))].sort());
const group = (folder:string) => vault.vaultResults.filter(n => (n.path.split('/').slice(0,-1).join('/') || '/') === folder);
function toggleFolder(folder:string) { const next = new Set(collapsed.value); next.has(folder) ? next.delete(folder) : next.add(folder); collapsed.value = next; }
const escape = (s:string) => s.replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const parser = new Marked({ extensions: [{ name:'wiki', level:'inline', start:src => src.indexOf('[['), tokenizer(src) {
  const m = /^\[\[([^\]\n]+)\]\]/.exec(src);
  if (m) return { type:'wiki', raw:m[0], target:m[1].split('|')[0], text:m[1].split('|')[1] || m[1] };
}, renderer(token:any) { return `<a href="${escape(encodeURI(token.target))}">${escape(token.text)}</a>`; } }] });
const html = computed(() => DOMPurify.sanitize(parser.parse(vault.vaultCurrent?.text || '') as string, { FORBID_TAGS:['img','iframe','object','form'], FORBID_ATTR:['style'] }));
const near = computed(() => new Set([vault.selected, ...vault.vaultEdges.filter(e => e.from === vault.selected || e.to === vault.selected).flatMap(e => [e.from,e.to])]));
const graphNodes = computed(() => [...near.value].filter(Boolean).slice(0,40).map(id => ({ id, kind:'note', label:id.split('/').pop()?.replace(/\.md$/i,'') || id })));
const graphEdges = computed(() => vault.vaultEdges.filter(e => graphNodes.value.some(n => n.id === e.from) && graphNodes.value.some(n => n.id === e.to)).map(e => ({ ...e, kind:'link' })));
const points = computed(() => new Map(layoutPositions(graphNodes.value as any, graphEdges.value as any).map(p => [p.id,p])));
function link(e:MouseEvent) {
  const a = (e.target as HTMLElement).closest('a'); if (!a) return;
  e.preventDefault(); vault.follow(a.getAttribute('href') || '');
}
watch(() => [vault.selected,vault.line,vault.anchor,vault.sourceMode], async () => {
  await nextTick();
  if (vault.sourceMode) reader.value?.querySelector(`[data-line="${vault.line}"]`)?.scrollIntoView({block:'center'});
  else if (vault.anchor) {
    const headings = [...(reader.value?.querySelectorAll('h1,h2,h3,h4,h5,h6') || [])];
    headings.find(h => h.textContent?.trim().toLowerCase() === vault.anchor.toLowerCase())?.scrollIntoView({block:'start'});
  } else if (reader.value) reader.value.scrollTop = 0;
});
</script>
<template>
<div class="vault-panel" :class="{ 'workbench-collapsed': workbench.collapsed }">
  <header class="vault-toolbar"><button :title="treeOpen ? '收起文件树' : '展开文件树'" @click="treeOpen = !treeOpen"><PanelLeftClose v-if="treeOpen" :size="16"/><PanelLeftOpen v-else :size="16"/></button><input v-model="vault.path" aria-label="Vault 目录" placeholder="Obsidian Vault 目录" @keydown.enter="vault.open()"/><button :disabled="vault.busy" title="选择 Vault 目录" @click="vault.pickDirectory()"><FolderOpen :size="16"/></button><button :disabled="vault.busy || !vault.path.trim()" title="打开 Vault" @click="vault.open()">打开</button><button :disabled="vault.busy || !vault.root" title="刷新 Vault" @click="vault.open()"><RefreshCw :size="16"/></button><button title="局部知识图谱" :aria-pressed="graphOpen" @click="graphOpen = !graphOpen"><Network :size="16"/></button><span>{{ vault.notes.length }} 笔记</span></header>
  <p v-if="vault.busy" role="status">正在读取 Vault…</p><p v-if="vault.error" role="alert" class="vault-error">{{ vault.error }}</p><p v-if="vault.warning" role="status">{{ vault.warning }}</p>
  <div v-if="vault.choices.length" class="vault-choices"><span>同名笔记：</span><button v-for="path in vault.choices" :key="path" @click="vault.select(path)">{{ path }}</button></div>
  <div class="vault-body">
    <aside v-if="treeOpen" class="vault-tree"><input v-model="vault.query" aria-label="搜索笔记" placeholder="搜索笔记与正文"/><div class="tree-actions"><button @click="collapsed = new Set()">全部展开</button><button @click="collapsed = new Set(folders)">全部折叠</button></div><template v-for="folder in folders" :key="folder"><button class="folder" :aria-expanded="!collapsed.has(folder)" @click="toggleFolder(folder)">{{ collapsed.has(folder) ? '▸' : '▾' }} {{ folder }}</button><div v-if="!collapsed.has(folder)"><button v-for="note in group(folder)" :key="note.path" class="note-row" :class="{selected:vault.selected === note.path}" @click="vault.select(note.path,note.line)">{{ note.path.split('/').pop() }}<small v-if="vault.query">L{{ note.line }} {{ note.snippet }}</small></button></div></template><p v-if="!vault.vaultResults.length">{{ vault.root ? '没有匹配笔记' : '尚未打开 Vault' }}</p><small v-if="vault.query && vault.vaultResults.length === 100">最多显示 100 项</small></aside>
    <section class="vault-document"><header><strong>{{ vault.selected || '笔记' }}</strong><div class="view-modes"><button :aria-pressed="!vault.sourceMode" @click="vault.sourceMode = false">阅读</button><button :aria-pressed="vault.sourceMode" @click="vault.sourceMode = true">原文</button></div></header><article ref="reader" class="vault-reader" @click="link"><pre v-if="vault.sourceMode"><span v-for="(text,i) in (vault.vaultCurrent?.text || '').split('\n')" :key="i" :data-line="i+1" :class="{hit: i+1 === vault.line}"><i>{{ i+1 }}</i>{{ text || ' ' }}
</span></pre><div v-else class="markdown" v-html="html"></div></article><footer v-if="vault.vaultBacklinks.length"><b>反向链接</b><button v-for="path in vault.vaultBacklinks.slice(0,30)" :key="path" @click="vault.select(path)">{{ path }}</button></footer></section>
    <aside v-if="graphOpen && vault.selected" class="vault-graph"><header>局部图谱 · {{ graphNodes.length }}</header><svg viewBox="0 0 800 600" aria-label="笔记关系图"><line v-for="(e,i) in graphEdges" :key="i" :x1="points.get(e.from)?.x" :y1="points.get(e.from)?.y" :x2="points.get(e.to)?.x" :y2="points.get(e.to)?.y" stroke="#8ba89e"/><g v-for="n in graphNodes" :key="n.id" :transform="`translate(${points.get(n.id)?.x},${points.get(n.id)?.y})`" tabindex="0" role="button" :aria-label="n.label" @click="vault.select(n.id)" @keydown.enter="vault.select(n.id)"><circle r="14" :fill="n.id === vault.selected ? '#148367' : '#cc8050'"/><text y="32" text-anchor="middle" font-size="16">{{ n.label.slice(0,22) }}</text><title>{{ n.id }}</title></g></svg><button v-for="n in graphNodes" :key="n.id" class="graph-link" @click="vault.select(n.id)">{{ n.label }}</button><small v-if="near.size > 40">局部图谱仅显示前 40 个节点</small></aside>
  </div>
</div>
</template>
<style scoped>
.workbench-collapsed .vault-tree,.workbench-collapsed .vault-graph{display:none}.vault-panel{height:100%;display:flex;flex-direction:column;color:#26343e;background:#fff;min-width:0}.vault-toolbar{display:flex;align-items:center;gap:6px;padding:6px 8px;border-bottom:1px solid #dce2e5}.vault-toolbar input{flex:1;min-width:100px}.vault-toolbar button{width:28px;height:28px;display:grid;place-items:center}.vault-toolbar span{font-size:12px;white-space:nowrap}input{padding:6px;border:1px solid #bccbd1;border-radius:4px;color:inherit;background:#fff}button{cursor:pointer;color:inherit;border:1px solid #d7dfe4;background:#f7f9fa;border-radius:4px;padding:4px 7px}button:disabled{opacity:.45;cursor:default}.vault-body{display:flex;flex:1;min-height:0;min-width:0}.vault-tree{width:230px;max-width:30%;flex:none;border-right:1px solid #dce2e5;overflow:auto;padding:8px;box-sizing:border-box}.vault-tree input{width:100%;box-sizing:border-box}.tree-actions{display:flex;gap:4px;margin:8px 0;font-size:12px}.folder,.note-row{display:block;width:100%;text-align:left;border:0;background:none;overflow-wrap:anywhere}.folder{font-weight:600;margin-top:6px}.note-row{padding:6px 8px}.selected{background:#e1f1e9}.note-row small{display:block;color:#647782;max-height:36px;overflow:hidden;font-size:11px}.vault-document{flex:1;min-width:0;display:flex;flex-direction:column}.vault-document header{display:flex;gap:8px;justify-content:space-between;padding:8px;border-bottom:1px solid #e5e9ec}.vault-document strong{overflow-wrap:anywhere;font-size:13px}.view-modes{display:flex;gap:4px;flex:none}[aria-pressed=true]{background:#e1f1e9}.vault-reader{flex:1;overflow:auto;padding:20px;min-height:0}.vault-reader pre{margin:0;font-size:13px;white-space:pre-wrap;overflow-wrap:anywhere}.vault-reader pre>span{display:block}.vault-reader i{display:inline-block;width:40px;color:#91a0a6;font-style:normal}.hit{background:#fff1bb}.markdown :deep(pre){background:#f1f4f5;padding:12px;overflow:auto;white-space:pre}.markdown :deep(a){color:#137861}.markdown :deep(table){border-collapse:collapse}.markdown :deep(td),.markdown :deep(th){border:1px solid #d2dbdf;padding:5px}.vault-document footer{display:flex;gap:8px;padding:8px;border-top:1px solid #dce2e5;flex-wrap:wrap;max-height:100px;overflow:auto;font-size:12px}.vault-graph{width:260px;max-width:25%;border-left:1px solid #dce2e5;overflow:auto;padding:8px}.vault-graph svg{width:100%;aspect-ratio:4/3}.vault-graph g{cursor:pointer}.graph-link{display:block;max-width:100%;margin:4px 0;overflow-wrap:anywhere}.vault-error{color:#af3535}.vault-panel>p{margin:4px 12px;font-size:12px}.vault-choices{padding:8px;display:flex;gap:8px;flex-wrap:wrap}@media(max-width:1000px){.vault-graph{width:190px}.vault-tree{width:190px}.vault-reader{padding:12px}}
</style>
