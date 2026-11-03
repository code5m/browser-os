<script setup lang="ts">
import {computed, onBeforeUnmount, onMounted, ref, watch} from 'vue';
import {useDatabaseStore} from '../state/useDatabaseStore';
import {buildResultView, decodeDbValue, truncateText} from '../../../utils/dbUi';
import {exportRows, assertLosslessQueryExport} from '../model/studioSql.mjs';
import {readDock, resizedOutput, writeDock, markdownResult} from '../model/ideaDock.mjs';

const db = useDatabaseStore();
const emit = defineEmits<{(e:'maximize', value:boolean):void}>();
type ViewMode = 'table'|'tree'|'text';
const saved=readDock();
const resultOpen=ref(saved.resultOpen);
const resultMaximized=ref(saved.resultMaximized);
const resultHeight=ref(saved.output);
const resultViewMode=ref<ViewMode>('table');
const transposed=ref(false);
const exportFormat=ref('csv');
const filter=ref('');
const page=ref(0);
const sortColumn=ref(-1);
const ascending=ref(true);
const hiddenColumns=ref<Set<number>>(new Set());
const valueEditor=ref<{name:string;value:string}|null>(null);
const notice=ref('');
const outputHost=ref<HTMLElement|null>(null);
const raw=computed(()=>db.document.raw);
const view=computed(()=>raw.value?buildResultView(raw.value):null);
// Preserve decoded scalar types for sorting, filtering, copying and export.
// The display projection may truncate text, but it never feeds an exporter.
const rawRows=computed(()=>Array.isArray(raw.value?.rows)?raw.value.rows.map(row=>(Array.isArray(row)?row:[]).map(cell=>decodeDbValue(cell))):[]);
const sourceLabel=computed(()=>{
  const conn=db.connections.find(c=>c.id===db.document.connId);
  return conn?.name||'未绑定数据源';
});
function compareCells(a:unknown,b:unknown):number {
  if(a===b)return 0;
  if(a===null||a===undefined)return -1;
  if(b===null||b===undefined)return 1;
  if(typeof a==='number'&&typeof b==='number')return a-b;
  if(typeof a==='boolean'&&typeof b==='boolean')return Number(a)-Number(b);
  return String(a).localeCompare(String(b),undefined,{numeric:true,sensitivity:'base'});
}
function displayCell(value:unknown):string {
  if(value===null||value===undefined)return 'NULL';
  if(typeof value==='boolean')return value?'true':'false';
  if(typeof value==='number')return String(value);
  return truncateText(String(value),200);
}
const rows=computed(()=>{
  const needle=filter.value.toLocaleLowerCase();
  const filtered=rawRows.value.filter(row=>row.some(cell=>String(cell??'NULL').toLocaleLowerCase().includes(needle)));
  if(sortColumn.value<0)return filtered;
  return [...filtered].sort((a,b)=>compareCells(a[sortColumn.value],b[sortColumn.value])*(ascending.value?1:-1));
});
const displayed=computed(()=>rows.value.slice(page.value*50,(page.value+1)*50));
const visibleColumns=computed(()=>view.value?.columns.map((name,index)=>({name,index})).filter(x=>!hiddenColumns.value.has(x.index))||[]);
const textPreview=computed(()=>{
  try{return exportText().slice(0,160000);}
  catch(error){return '无法生成文本预览：'+String(error);}
});
watch(()=>[db.activeDocument,raw.value,filter.value],()=>{
  page.value=0;sortColumn.value=-1;hiddenColumns.value=new Set();valueEditor.value=null;
  if(raw.value){resultOpen.value=true;saveResultDock();}
});
function saveResultDock(){
  writeDock({...readDock(),output:resultHeight.value,resultOpen:resultOpen.value,resultMaximized:resultMaximized.value});
}
function toggleResults(){
  resultOpen.value=!resultOpen.value;
  if(!resultOpen.value && resultMaximized.value){resultMaximized.value=false;emit('maximize',false);}
  saveResultDock();
}
function toggleMaximized(){
  if(!resultOpen.value)resultOpen.value=true;
  resultMaximized.value=!resultMaximized.value;
  emit('maximize',resultMaximized.value);
  saveResultDock();
}
let clearOutputDrag=()=>{};
function onOutputPointerDown(event:PointerEvent){
  if(event.button!==0 || !outputHost.value || !resultOpen.value || resultMaximized.value)return;
  event.preventDefault();
  clearOutputDrag();
  const initial=resultHeight.value;
  const start=event.clientY;
  const available=outputHost.value.parentElement?.getBoundingClientRect().height||700;
  const move=(next:PointerEvent)=>{resultHeight.value=resizedOutput(initial,next.clientY-start,available);};
  const finish=()=>{clearOutputDrag();saveResultDock();};
  const clear=()=>{
    window.removeEventListener('pointermove',move);
    window.removeEventListener('pointerup',finish);
    window.removeEventListener('pointercancel',finish);
    window.removeEventListener('blur',finish);
  };
  clearOutputDrag=()=>{clearOutputDrag=()=>{};clear();};
  window.addEventListener('pointermove',move);window.addEventListener('pointerup',finish);
  window.addEventListener('pointercancel',finish);window.addEventListener('blur',finish);
}
function onOutputKeyDown(event:KeyboardEvent){
  if(event.key!=='ArrowUp'&&event.key!=='ArrowDown')return;
  event.preventDefault();
  resultHeight.value=resizedOutput(resultHeight.value,event.key==='ArrowUp'?-16:16,outputHost.value?.parentElement?.clientHeight||700);
  saveResultDock();
}
function sort(index:number){ascending.value=index===sortColumn.value?!ascending.value:true;sortColumn.value=index;page.value=0;}
function toggleColumn(index:number){
  const next=new Set(hiddenColumns.value);
  if(next.has(index))next.delete(index);
  else if(next.size < (view.value?.columns.length||0)-1)next.add(index);
  hiddenColumns.value=next;
}
function exportText(){
  const v=view.value;
  if(!v)return '';
  assertLosslessQueryExport(raw.value?.rows ?? []);
  if(exportFormat.value==='markdown')return markdownResult(v.columns,rows.value);
  return exportRows(v.columns,rows.value,exportFormat.value);
}
async function copyData(){
  try{await navigator.clipboard.writeText(exportText());notice.value='已复制当前读取并筛选的结果';}
  catch(error){notice.value='复制失败：'+String(error);}
}
function downloadData(){
  try{
    const contents=exportText();
    const ext=exportFormat.value==='markdown'?'md':exportFormat.value;
    const blob=new Blob([ext==='csv'||ext==='tsv'?'\uFEFF'+contents:contents],{type:'text/plain;charset=utf-8'});
    const url=URL.createObjectURL(blob);
    const link=document.createElement('a');
    link.href=url;link.download='browseros-result-'+new Date().toISOString().slice(0,10)+'.'+ext;
    link.click();
    setTimeout(()=>URL.revokeObjectURL(url),1000);
    notice.value='已导出当前已读取、筛选的结果（不会查询更多数据）';
  }catch(error){notice.value='导出失败：'+String(error);}
}
function openCell(name:string,value:unknown){valueEditor.value={name,value:value===null||value===undefined?'NULL':String(value)};}
onMounted(()=>emit('maximize',resultMaximized.value));
onBeforeUnmount(()=>{clearOutputDrag();emit('maximize',false);});
</script>

<template>
<div ref="outputHost" class="db-result-dock" :class="{'is-collapsed':!resultOpen,'is-maximized':resultMaximized}" :style="{'--db-result-height':resultHeight+'px'}">
  <div v-if="resultOpen&&!resultMaximized" class="db-output-splitter" role="separator" tabindex="0" aria-label="拖动调整 Output 高度" aria-orientation="horizontal" :aria-valuenow="resultHeight" @pointerdown="onOutputPointerDown" @keydown="onOutputKeyDown"></div>
  <section class="ide-result-pane" aria-label="SQL Output 结果面板">
    <header class="ide-output-head">
      <strong>▤ Output</strong>
      <span class="output-source" :title="sourceLabel">{{sourceLabel}}</span>
      <span v-if="view" class="output-status">{{view.rowLabel}} · {{view.elapsedLabel}}</span>
      <span class="output-spacer"></span>
      <button type="button" :aria-pressed="resultMaximized" :title="resultMaximized?'退出最大化':'最大化 Output'" @click="toggleMaximized">{{resultMaximized?'▣ 还原':'□ 最大化'}}</button>
      <button type="button" :aria-expanded="resultOpen" @click="toggleResults">{{resultOpen?'⌄ 收起':'⌃ 展开'}}</button>
    </header>
    <div v-if="resultOpen" class="ide-output-body">
      <nav class="db-result-toolbar" aria-label="数据结果操作">
        <label>Data Extractors
          <select v-model="exportFormat" class="db-result-extractors" aria-label="数据导出器">
            <option value="csv">CSV</option><option value="tsv">TSV</option><option value="json">JSON</option>
            <option value="markdown">Markdown</option>
          </select>
        </label>
        <button type="button" :disabled="!view" @click="copyData">复制</button>
        <button type="button" :disabled="!view" @click="downloadData">导出</button>
        <span class="output-divider"></span>
        <button v-for="mode in (['table','tree','text'] as const)" :key="mode" type="button" :aria-pressed="resultViewMode===mode" @click="resultViewMode=mode">{{mode==='table'?'Table':mode==='tree'?'Tree':'Text'}}</button>
        <button type="button" :disabled="resultViewMode!=='table'||!view" :aria-pressed="transposed" title="交换字段和记录的方向" @click="transposed=!transposed">⇄ 转置</button>
        <details v-if="view" class="db-column-menu"><summary>列 ▾</summary>
          <label v-for="(column,index) in view.columns" :key="index"><input type="checkbox" :checked="!hiddenColumns.has(index)" :disabled="visibleColumns.length===1&&!hiddenColumns.has(index)" @change="toggleColumn(index)"/>{{column}}</label>
        </details>
        <input v-model="filter" type="search" class="db-output-filter" aria-label="本地筛选已读取结果" placeholder="筛选已读取行…" />
      </nav>
      <p v-if="notice" class="output-note" role="status">{{notice}}</p>
      <nav v-if="db.document.batchResults?.length>1" class="db-batch-tabs" aria-label="多语句执行结果"><button v-for="(_,index) in db.document.batchResults" :key="index" @click="db.selectBatchResult(index)">结果 {{index+1}}</button></nav>
      <div v-if="view" class="ide-result-content">
        <p v-for="warning in view.warnings" :key="warning" class="output-warning">{{warning}}</p>
        <div v-if="resultViewMode==='table'" class="db-result-scroller">
          <table v-if="!transposed" class="db-result-table">
            <thead><tr><th class="db-row-number">#</th><th v-for="column in visibleColumns" :key="column.index"><button type="button" @click="sort(column.index)">{{column.name}}{{sortColumn===column.index?(ascending?' ↑':' ↓'):''}}</button></th></tr></thead>
            <tbody><tr v-for="(row,index) in displayed" :key="index"><th class="db-row-number">{{page*50+index+1}}</th><td v-for="column in visibleColumns" :key="column.index" :title="String(row[column.index]??'')" @dblclick="openCell(column.name,row[column.index])">{{displayCell(row[column.index])}}</td></tr></tbody>
          </table>
          <table v-else class="db-result-table db-transposed">
            <thead><tr><th>字段 / 行</th><th v-for="(row,index) in displayed" :key="index">{{page*50+index+1}}</th></tr></thead>
            <tbody><tr v-for="column in visibleColumns" :key="column.index"><th>{{column.name}}</th><td v-for="(row,index) in displayed" :key="index" @dblclick="openCell(column.name,row[column.index])">{{displayCell(row[column.index])}}</td></tr></tbody>
          </table>
        </div>
        <div v-else-if="resultViewMode==='tree'" class="db-result-tree">
          <details v-for="(row,index) in displayed" :key="index"><summary>▦ Row {{page*50+index+1}}</summary><dl><div v-for="column in visibleColumns" :key="column.index"><dt>{{column.name}}</dt><dd @dblclick="openCell(column.name,row[column.index])">{{displayCell(row[column.index])}}</dd></div></dl></details>
        </div>
        <pre v-else class="db-result-text">{{textPreview}}</pre>
        <footer class="db-result-footer">
          <span>仅本地排序、筛选已读取 {{view.rows.length}} 行；未读取全库。</span>
          <button type="button" :disabled="page===0" @click="page--">上一页</button>
          <span>{{page+1}} / {{Math.max(1,Math.ceil(rows.length/50))}}</span>
          <button type="button" :disabled="(page+1)*50>=rows.length" @click="page++">下一页</button>
        </footer>
      </div>
      <p v-else class="db-no-result">尚无查询结果。执行只读 SQL 后，这里显示真实结果；不会创建演示记录。</p>
      <details class="db-output-sessions"><summary>任务 / 会话信息</summary>
        <span>原生桥接：{{db.backendReady?'已连接':'未就绪'}}</span>
        <span>当前事务：{{db.document.transaction||'idle'}}</span>
        <span>会话：{{db.document.sessionId?'独立会话':'按需连接'}}</span>
      </details>
      <aside v-if="valueEditor" class="db-value-overlay" role="dialog" aria-modal="true" aria-label="只读单元格值">
        <div><strong>Value Editor · {{valueEditor.name}}</strong><button @click="valueEditor=null" aria-label="关闭值查看器">×</button></div>
        <textarea :value="valueEditor.value" rows="6" readonly spellcheck="false" aria-label="只读单元格内容"/>
        <p>仅查看当前读取值。编辑或写入须使用受控表编辑器。</p>
      </aside>
    </div>
  </section>
</div>
</template>

<style scoped>
.db-result-dock{position:relative;display:flex;flex-direction:column;flex:0 0 var(--db-result-height,270px);min-height:0;min-width:0;background:#151a23;border-top:1px solid #334054}
.db-result-dock.is-collapsed{flex:0 0 35px}
.db-result-dock.is-maximized{flex:1 1 100%;min-height:0}
.db-output-splitter{flex:0 0 6px;background:#283142;cursor:row-resize;touch-action:none;position:relative;z-index:2}
.db-output-splitter:hover,.db-output-splitter:focus-visible{background:#559ffc;outline:0}
.db-output-splitter:before{content:"";position:absolute;left:46%;right:46%;top:2px;bottom:2px;border-radius:2px;background:#708198}
.ide-result-pane{display:flex;flex-direction:column;min-height:0;min-width:0;flex:1;overflow:hidden}
.ide-output-head{display:flex;align-items:center;min-height:30px;gap:8px;padding:2px 9px;border-bottom:1px solid #303a49;background:#202631;color:#b8c8dc;font-size:11px}
.ide-output-head strong{color:#f1f5fb}
.output-source{overflow:hidden;white-space:nowrap;text-overflow:ellipsis;max-width:34%}.output-status{color:#8fa6be}.output-spacer{flex:1}
.ide-output-head button{border:0;background:transparent;color:#b9c7d8;padding:3px 7px}
.ide-output-head button:hover{background:#32435a}
.ide-output-body{display:flex;flex-direction:column;position:relative;flex:1;min-height:0;overflow:hidden}
.db-result-toolbar{display:flex;align-items:center;flex-wrap:wrap;gap:4px;padding:4px 8px;border-bottom:1px solid #30394a;background:#191f29;font-size:11px}
.db-result-toolbar label{display:inline-flex;gap:5px;align-items:center}
.db-result-toolbar button,.db-result-toolbar select,.db-result-toolbar input{background:transparent;border:1px solid #354258;border-radius:3px;color:#bdcadd;font-size:11px;padding:4px 6px}
.db-result-toolbar button[aria-pressed=true]{background:#334e74;color:#fff;border-color:#577bac}
.db-result-toolbar button:disabled{opacity:.4}.db-result-toolbar button:hover:not(:disabled){background:#2a3c55}
.output-divider{width:1px;height:19px;background:#425169;margin:0 5px}
.db-result-toolbar .db-output-filter{min-width:110px;flex:1;max-width:280px;margin-left:auto}
.db-column-menu{position:relative}.db-column-menu summary{list-style:none;cursor:pointer;border:1px solid #354258;border-radius:3px;padding:4px 8px}
.db-column-menu[open]>label{display:flex;gap:6px;padding:4px 8px;background:#212b3b}
.db-column-menu[open]{position:relative;z-index:4;min-width:130px}
.db-column-menu[open] summary{background:#344762}
.db-column-menu label input{width:auto}
.db-result-content,.ide-result-content{display:flex;flex-direction:column;flex:1;min-height:0;overflow:hidden}
.db-result-scroller,.db-result-tree,.db-result-text{flex:1;min-height:0;overflow:auto;scrollbar-width:thin}
.db-result-table{border-collapse:collapse;width:max-content;min-width:100%;font:12px/1.55 ui-monospace,monospace;color:#cfd9e8}
.db-result-table th,.db-result-table td{padding:4px 10px;border:1px solid #2c3544;white-space:pre;max-width:400px;overflow:hidden;text-overflow:ellipsis}
.db-result-table thead th{position:sticky;top:0;z-index:1;background:#252e3b;color:#b8cbe4;text-align:left}
.db-result-table th button{background:transparent;border:0;color:inherit;padding:0;cursor:pointer}
.db-result-table tbody tr:hover td{background:#273750}.db-result-table td:focus,.db-result-table td:active{background:#304f78}
.db-row-number{position:sticky;left:0;min-width:35px;width:40px;background:#202836;color:#8394aa!important;font-weight:400;text-align:right!important}
.db-transposed tbody th{position:sticky;left:0;background:#252e3b;color:#bdcbe0}
.db-result-tree details{border-bottom:1px solid #2b3545;padding:6px 11px;color:#d9e2ed}
.db-result-tree summary{cursor:pointer;font-weight:600}.db-result-tree dl{padding:2px 8px;margin:5px 0}.db-result-tree dl>div{display:flex;gap:12px;padding:5px;border-bottom:1px solid #293242}
.db-result-tree dt{width:34%;color:#8da9cb}.db-result-tree dd{margin:0;word-break:break-all}
.db-result-text{margin:0;white-space:pre;overflow:auto;padding:12px;background:#111823;color:#cbd7e6;font:12px/1.55 ui-monospace,monospace}
.db-result-footer{display:flex;align-items:center;gap:7px;min-height:30px;padding:3px 8px;background:#1b2331;color:#8ea0b6;font-size:10px}
.db-result-footer>span:first-child{flex:1}
.db-result-footer button{border:1px solid #354258;border-radius:3px;padding:2px 7px;background:#273247;color:#cbd5e1}
.db-result-footer button:disabled{opacity:.4}
.db-no-result{flex:1;padding:26px;color:#899eb8}
.output-note,.output-warning{font-size:11px;margin:0;padding:4px 10px;color:#a6caff;max-height:50px;overflow:auto}
.output-warning{color:#f0bd6a}
.db-output-sessions{font-size:10px;flex:none;border-top:1px solid #30394a;padding:3px 8px;color:#8ea0b6}
.db-output-sessions[open] span{display:inline-block;margin-right:14px;padding:5px}
.db-batch-tabs{display:flex;flex-wrap:wrap;gap:4px;padding:3px 9px}
.db-batch-tabs button{background:#273247;border:0;border-radius:3px;color:#b8c8dd;padding:3px 8px}
.db-value-overlay{position:absolute;z-index:5;left:12%;right:12%;top:16%;padding:12px;border:1px solid #5b7898;border-radius:8px;background:#1c293a;box-shadow:0 14px 36px #0009}
.db-value-overlay>div{display:flex;align-items:center;justify-content:space-between;gap:7px}
.db-value-overlay button{border:0;background:transparent;color:#fff;cursor:pointer;font-size:17px}
.db-value-overlay textarea{display:block;width:100%;box-sizing:border-box;resize:vertical;padding:9px;color:#d9e4f4;background:#111823;border:1px solid #3b4d65;font:12px ui-monospace,monospace}
.db-value-overlay p{margin:7px 0 0;color:#a7bacf;font-size:10px}
@media(max-width:850px){.db-result-toolbar .db-output-filter{flex-basis:100%;max-width:none}.db-result-toolbar label{font-size:10px}}
</style>
