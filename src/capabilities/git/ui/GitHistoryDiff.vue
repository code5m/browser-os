<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue';
import { ArrowUp, ArrowDown, ChevronsUpDown } from '@lucide/vue';
import { historyContextRows, type DiffRow } from '../model/patchView';
const props=defineProps<{patch:string;afterContent:string;onlyChanges:boolean;beforeLabel:string;afterLabel:string}>();
const root=ref<HTMLElement|null>(null), expanded=ref(new Set<string>()), active=ref(-1);
const full=computed(()=>historyContextRows(props.patch,props.afterContent));
type Row=DiffRow & {fold?:number;key:string;change:number};
const rows=computed(()=>{
  const output:Row[]=[];let block=-1, wasChange=false;
  for(let index=0;index<full.value.length;){
    const row=full.value[index];
    if(row.kind==='context') {
      let end=index;while(end<full.value.length && full.value[end].kind==='context')end++;
      const key=`${index}:${end}`;
      const count=end-index;
      if(props.onlyChanges && count>6 && !expanded.value.has(key)) {
        for(let i=index;i<index+3;i++)output.push({...full.value[i],key:String(i),change:-1});
        output.push({...row,before:'',after:'',beforeLine:null,afterLine:null,fold:count-6,key,change:-1});
        for(let i=end-3;i<end;i++)output.push({...full.value[i],key:String(i),change:-1});
      } else for(let i=index;i<end;i++)output.push({...full.value[i],key:String(i),change:-1});
      wasChange=false;index=end;
    } else {
      if(!wasChange)block++;
      output.push({...row,key:String(index),change:block});wasChange=true;index++;
    }
  }
  return output;
});
const blocks=computed(()=>Math.max(-1,...rows.value.map(row=>row.change))+1);
watch(()=>props.patch,()=>{expanded.value=new Set();active.value=-1;});
function expand(key:string){expanded.value=new Set([...expanded.value,key]);}
async function move(delta:number){
  if(!blocks.value)return;
  active.value=Math.max(0,Math.min(blocks.value-1,active.value+delta));
  await nextTick();
  const panes=root.value?.querySelectorAll<HTMLElement>('.diff-side');
  const pane=panes?.[0], row=pane?.querySelector<HTMLElement>(`[data-change="${active.value}"]`);
  if(!pane || !row || !panes)return;
  const target=Math.max(0,pane.scrollTop+row.getBoundingClientRect().top-pane.getBoundingClientRect().top-pane.clientHeight/2+row.clientHeight/2);
  for(const side of panes)side.scrollTop=target;
}
function keyboard(event:KeyboardEvent){if(event.key==='F7'){event.preventDefault();void move(event.shiftKey?-1:1);}}
function syncScroll(event:Event){const source=event.currentTarget as HTMLElement;for(const pane of root.value?.querySelectorAll<HTMLElement>('.diff-side') || [])if(pane!==source && pane.scrollTop!==source.scrollTop)pane.scrollTop=source.scrollTop;}
</script>
<template>
  <section ref="root" class="history-diff" tabindex="0" aria-label="双栏历史差异" @keydown="keyboard">
    <header class="diff-controls"><button title="上一个差异（Shift+F7）" aria-label="上一个差异" :disabled="!blocks || active<=0" @click="move(-1)"><ArrowUp :size="15"/></button><button title="下一个差异（F7）" aria-label="下一个差异" :disabled="!blocks || active>=blocks-1" @click="move(1)"><ArrowDown :size="15"/></button><span>{{active<0 ? blocks+' 个变更块' : (active+1)+' / '+blocks}}</span><span class="readonly">历史版本 · 只读</span></header>
    <div class="diff-versions"><span>{{beforeLabel}}</span><span>{{afterLabel}}</span></div>
    <div class="diff-viewport"><div v-for="side in ['before','after'] as const" :key="side" class="diff-side" :aria-label="side==='before'?'修改前':'修改后'" @scroll="syncScroll">
      <template v-for="row in rows" :key="row.key">
        <button v-if="row.fold" class="context-fold" @click="expand(row.key)" :aria-label="'展开 '+row.fold+' 行未变更内容'"><ChevronsUpDown :size="13"/>展开 {{row.fold}} 行未变更内容</button>
        <div v-else class="code-line" :data-change="row.change>=0?row.change:undefined" :class="{removed:side==='before'&&row.removed,added:side==='after'&&row.added,focused:row.change===active&&active>=0,blank:side==='before'?row.beforeLine===null:row.afterLine===null}"><span class="line-number">{{side==='before'?row.beforeLine:row.afterLine}}</span><code>{{side==='before'?row.before:row.after}}</code></div>
      </template>
    </div></div>
    <p v-if="!rows.length" class="diff-empty">此文件没有可显示的文本差异。</p>
  </section>
</template>
<style scoped>
.history-diff{display:flex;flex:1;flex-direction:column;min-width:0;min-height:0;height:100%;color:#d8dee9;background:#20242b;outline:none;font:12px/1.6 ui-monospace,SFMono-Regular,Consolas,monospace}
.diff-controls{display:flex;align-items:center;gap:7px;padding:5px 9px;border-bottom:1px solid #353b45;font:11px system-ui,sans-serif;background:#262b33}.diff-controls button{display:grid;place-items:center;border:0;border-radius:4px;width:26px;height:24px;background:transparent;color:inherit;cursor:pointer}.diff-controls button:hover{background:#3a4352}.diff-controls button:disabled{opacity:.35;cursor:default}.readonly{margin-left:auto;color:#929cab}
.diff-versions{display:grid;grid-template-columns:1fr 1fr;border-bottom:1px solid #343b46;color:#b1bdcf;background:#252b33;font:11px system-ui,sans-serif}.diff-versions span{padding:7px 12px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.diff-versions span+span{border-left:1px solid #465363}
.diff-viewport{display:flex;overflow:hidden;min-height:0;flex:1;align-items:stretch}.diff-side{width:50%;min-width:0;overflow:auto;flex:none;border-right:1px solid #465363;box-sizing:border-box}.code-line{display:flex;min-width:max-content;height:23px;box-sizing:border-box;white-space:pre;align-items:center}.code-line code{font:inherit;line-height:23px;padding:0 12px 0 8px;tab-size:4}.line-number{display:inline-block;flex:none;min-width:43px;box-sizing:border-box;padding:0 8px;color:#8491a5;text-align:right;user-select:none;border-right:1px solid #343c49}.code-line.removed{background:#4b3037;color:#ffdee3}.code-line.added{background:#1f3a2e;color:#d2efd9}.code-line.blank{background:#292f37}.code-line.focused{box-shadow:inset 0 1px #739bd1,inset 0 -1px #739bd1}
.context-fold{display:flex;align-items:center;justify-content:center;gap:6px;width:100%;height:27px;min-height:27px;border:0;border-top:1px solid #3c485a;border-bottom:1px solid #3c485a;background:#293649;color:#b3cee9;cursor:pointer;font:11px system-ui,sans-serif}.context-fold:hover{background:#354c66}.diff-empty{padding:15px;color:#9da9ba}
</style>
