<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue';
import { RefreshCw, Copy } from '@lucide/vue';
import { bridge } from '../../../bridge';
import { useGitStore } from '../../../capabilities/git/public';
import { layoutCommitGraph } from '../model/logGraph';
import { redactSecrets } from '../../../utils/redact';
const git = useGitStore();
const commits = ref<Awaited<ReturnType<typeof bridge.gitLog>>>([]);
const selected = ref('');
const diff = ref('');
const query = ref('');
const loading = ref(false);
const error = ref('');
let generation = 0;
let diffGeneration = 0;
const annotated = computed(() => {
  const graph = layoutCommitGraph(commits.value);
  return commits.value.map((commit, index) => ({...commit,graph:graph[index]}));
});
const filtered = computed(() => annotated.value.filter(c =>
  `${c.summary} ${c.author} ${c.oid}`.toLowerCase().includes(query.value.toLowerCase())
));
async function refresh() {
  const repoId = git.repoId; const request = ++generation; ++diffGeneration;
  commits.value = []; selected.value = ''; diff.value = ''; error.value = '';
  if (!repoId) { loading.value = false; return; }
  loading.value = true;
  try { const data = await bridge.gitLog(repoId); if (request === generation) commits.value = data; }
  catch(e) { if (request === generation) error.value = redactSecrets(e); }
  finally { if (request === generation) loading.value = false; }
}
async function select(oid:string) {
  const repoId = git.repoId; if (!repoId) return;
  const request = ++diffGeneration; selected.value = oid; diff.value = ''; error.value = '';
  try { const data = await bridge.gitCommitDiff(repoId,oid); if (request === diffGeneration) diff.value = data || '此提交没有文本差异'; }
  catch(e) { if (request === diffGeneration) error.value = redactSecrets(e); }
}
async function copy() { try { await navigator.clipboard.writeText(selected.value); } catch { error.value = '复制失败'; } }
watch(() => [git.repoId,git.lastJob?.id,git.busy], refresh, {immediate:true});
onUnmounted(() => { generation++; diffGeneration++; });
</script>
<template><section class="history"><header><input v-model="query" aria-label="搜索提交" placeholder="搜索提交、作者、哈希"/><button title="刷新历史" :disabled="loading || !git.repoId" @click="refresh"><RefreshCw :size="14"/></button><span>{{ commits.length }} 条提交</span></header><p v-if="error" role="alert">{{ error }}</p><div class="history-body"><div class="commits"><p v-if="loading">正在读取历史…</p><p v-else-if="!filtered.length">没有匹配的提交</p><button v-for="c in filtered" :key="c.oid" :class="{selected:selected === c.oid}" @click="select(c.oid)"><svg class="commit-graph" viewBox="0 0 150 36" xmlns="http://www.w3.org/2000" role="img" :aria-label="'提交图谱：'+(c.graph?.hasMerge?'合并提交':'普通提交')">
  <line v-for="lane in c.graph?.through || []" :key="'s'+lane" :x1="10+lane*14" y1="0" :x2="10+lane*14" y2="36" stroke="#91b2d2" stroke-width="2"/>
  <line v-for="(lane,i) in c.graph?.parentLanes || []" :key="'p'+i"
    :x1="10+(c.graph?.lane||0)*14" y1="18" :x2="10+lane*14" y2="36" stroke="#5187c5" stroke-width="2"/>
  <circle :cx="10+(c.graph?.lane||0)*14" cy="17" r="4.5" :fill="c.graph?.hasMerge?'#a269bd':'#286fc0'"/>
</svg><div><strong>{{ c.summary }}</strong><small>{{ c.oid.slice(0,8) }} · {{ c.author }} · {{ new Date(c.time*1000).toLocaleDateString() }}</small></div></button><small v-if="commits.length === 200">图谱只显示最近 200 条；视图之外的祖先不会推断或伪造。</small></div><div class="commit-diff"><header v-if="selected"><code>{{ selected }}</code><button title="复制提交哈希" @click="copy"><Copy :size="14"/></button></header><pre>{{ selected ? diff || '读取差异中…' : '选择提交查看差异' }}</pre></div></div></section></template>
<style scoped>.history{display:flex;flex-direction:column;min-height:0;flex:1;background:#fff;color:#293c44}.history header{display:flex;align-items:center;gap:8px;padding:8px;border-bottom:1px solid #dde5e8;font-size:12px}.history input{flex:1;min-width:0;padding:5px;border:1px solid #bbcbd2;border-radius:4px}.history button{border:0;background:none;color:inherit;cursor:pointer}.history header button{display:grid;place-items:center}.history-body{display:flex;flex:1;min-height:0}.commits{width:35%;min-width:230px;overflow:auto;border-right:1px solid #dde5e8}.commits>button{display:flex;gap:10px;text-align:left;width:100%;padding:8px;border-bottom:1px solid #edf1f2}.commit-mark{color:#21816b}.commits strong{display:block;overflow-wrap:anywhere;font-size:13px;font-weight:500}.commits small{display:block;color:#768892;font-size:11px;padding-top:4px}.commits .selected{background:#e6f3ed}.commit-diff{flex:1;min-width:0;display:flex;flex-direction:column}.commit-diff code{overflow-wrap:anywhere}.commit-diff pre{flex:1;overflow:auto;padding:12px;margin:0;font-size:12px;line-height:1.6;background:#f7f9fa}.history>p{color:#a03535;margin:8px}.commit-graph{width:85px;min-width:85px;height:36px;flex-shrink:0}.commits>button{align-items:center}.commits>button>div{min-width:0}
</style>
