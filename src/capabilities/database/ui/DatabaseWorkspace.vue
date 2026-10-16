<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useDatabaseStore } from '../state/useDatabaseStore';
import { buildResultView, decodeDbValue, riskLabel, productionLabel } from '../../../utils/dbUi';
import { exportRows } from '../model/studioSql.mjs';
import SqlEditor from './SqlEditor.vue';
import TableDataEditor from './TableDataEditor.vue';
const db=useDatabaseStore();
const filter=ref(''),page=ref(0),sortColumn=ref(-1),ascending=ref(true),historyOpen=ref(false);
const exportFormat=ref('csv'),pendingTransaction=ref<{document:string;action:string;writable?:boolean}|null>(null),acknowledged=ref(false),notice=ref('');
const connection=computed(()=>db.connections.find(c=>c.id===db.document.connId));
const config=computed(()=>db.configs.find(c=>c.id===db.document.connId));
const sourceLabel=computed(()=>connection.value?`${connection.value.name} · ${config.value?.database||''}`:'请从左侧选择数据源');
const columns=computed(()=>db.objectDetailMode==='columns'?db.objectDetailRows.map(row=>row[0]):[]);
const raw=computed(()=>db.document.raw);
const view=computed(()=>raw.value?buildResultView(raw.value):null);
const rows=computed(()=>{
  const items=(view.value?.rows||[]).filter(row=>row.some(cell=>cell.toLowerCase().includes(filter.value.toLowerCase())));
  return sortColumn.value<0?items:[...items].sort((a,b)=>String(a[sortColumn.value]??'').localeCompare(String(b[sortColumn.value]??''),undefined,{numeric:true})*(ascending.value?1:-1));
});
const displayed=computed(()=>rows.value.slice(page.value*50,(page.value+1)*50));
watch(()=>[db.activeDocument,db.document.raw,filter.value],()=>{page.value=0;pendingTransaction.value=null;acknowledged.value=false;});
function run(request:{text:string;batch:boolean}){void db.requestRunText(request.text,request.batch);}
function transaction(action:string,writable=false){pendingTransaction.value={document:db.activeDocument,action,writable};acknowledged.value=false;}
async function confirmTransaction(){const pending=pendingTransaction.value;if(!pending||!acknowledged.value||pending.document!==db.activeDocument)return;pendingTransaction.value=null;await db.sessionAction(pending.action,{writable:pending.writable,confirmed:true});}
async function closeSession(){
  const owner=db.document;
  notice.value='正在关闭会话资源，不会自动提交事务。';
  const closed=await db.releaseSession(owner);
  if(db.activeDocument!==owner.id)return;
  notice.value=closed?'会话资源已关闭。如果事务结果未知，仍需核对数据库，不能据此认定已回滚。':'关闭尚未确认，请重试关闭；不要重复写入。';
}
function sort(index:number){ascending.value=index===sortColumn.value?!ascending.value:true;sortColumn.value=index;}
function exportText(){if(!raw.value)return '';return exportRows(raw.value.columns,raw.value.rows.map(row=>row.map(decodeDbValue)),exportFormat.value);}
async function copy(){try{await navigator.clipboard.writeText(exportText());notice.value='已复制当前已读取结果。';}catch(e){notice.value=`复制失败：${String(e)}`;}}
function download(){try{const text=exportText();const url=URL.createObjectURL(new Blob([exportFormat.value==='json'?text:'\uFEFF'+text],{type:exportFormat.value==='json'?'application/json':'text/plain;charset=utf-8'}));const link=document.createElement('a');link.href=url;link.download=`browseros-query-${new Date().toISOString().slice(0,10)}.${exportFormat.value}`;link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}catch(e){notice.value=`导出失败：${String(e)}`;}}
function selectResult(index:number){db.selectBatchResult(index);}
</script>
<template>
<section class="database-workspace" aria-label="SQL 查询工作区">
  <div class="documents" role="tablist"><div v-for="document in db.documents" :key="document.id" class="document-tab" :class="{active:document.id===db.activeDocument}"><button role="tab" :aria-selected="document.id===db.activeDocument" :title="db.connections.find(c=>c.id===document.connId)?.name||'未绑定数据源'" @click="db.activateDocument(document.id)">{{document.name}}<span v-if="document.transaction&&document.transaction!=='idle'"> ●</span><span v-if="document.busy"> · 执行中</span></button><button :disabled="document.busy" :aria-label="'关闭 '+document.name" @click="db.closeDocument(document.id)">×</button></div><button @click="db.newDocument()">＋ 查询</button></div>
  <div v-if="db.closePending" class="confirmation">关闭会丢弃本页 SQL 草稿，并回滚本页未提交事务。<button @click="db.closeDocument(db.closePending!,true)">回滚并关闭页签</button><button @click="db.closePending=null">保留页签</button></div>
  <header class="session"><strong>{{sourceLabel}}</strong><span>{{db.document.sessionId?'独立原生会话':'按需连接'}} · 事务：{{db.document.transaction||'idle'}}</span><button :disabled="db.busy||!db.document.connId||db.document.transaction!=='idle'" @click="db.sessionAction('begin',{writable:false,confirmed:false})">只读事务</button><button :disabled="db.busy||!db.document.connId||db.document.transaction!=='idle'" @click="transaction('begin',true)">写事务…</button><button :disabled="db.busy||!db.document.sessionId||db.document.transaction==='idle'||['failed','unknown'].includes(db.document.transaction)" @click="transaction('commit')">提交事务…</button><button :disabled="db.busy||!db.document.sessionId||db.document.transaction==='idle'" @click="transaction('rollback')">回滚事务…</button><button :disabled="!db.document.sessionId" @click="closeSession">关闭会话</button></header>
  <p class="source-note">每页 SQL 固定绑定自己的数据源；选择另一个数据源不会把已有 SQL 静默改到新库。最多 4 个原生会话，空闲 5 分钟自动回滚并释放。</p>
  <div v-if="pendingTransaction" class="confirmation"><strong>{{pendingTransaction.action==='commit'?'确认提交本会话的事务':pendingTransaction.action==='rollback'?'确认回滚本会话的未提交修改':'确认开启允许写入的事务'}}</strong><p>{{sourceLabel}}；生产环境及未知环境仍由原生安全策略拒绝写入。</p><label><input v-model="acknowledged" type="checkbox"/>我已核对当前页的数据源与事务</label><button :disabled="!acknowledged||db.busy" @click="confirmTransaction">确认</button><button @click="pendingTransaction=null">取消</button></div>
  <TableDataEditor v-if="db.document.tableData"/>
  <SqlEditor v-else :model-value="db.sql" :disabled="db.document.busy||db.confirmOpen||db.document.closing||db.document.closeFailed||db.document.transaction==='unknown'" :objects="db.schema.map(item=>item.name)" :columns="columns" :source-label="sourceLabel" @update:model-value="db.sql=$event" @run="run"/>
  <div class="actions"><button :disabled="!db.document.busy" @click="db.cancelQuery()">取消查询</button><button :disabled="db.busy||!db.sql.trim()||config?.kind!=='mysql'" @click="db.explainCurrentSql()">EXPLAIN SELECT</button><button :disabled="!raw&&!db.error" @click="db.clearResult()">清空结果</button><button :aria-pressed="historyOpen" @click="historyOpen=!historyOpen">查询历史</button><span>{{riskLabel(db.risk)}} · {{productionLabel(db.verdict)}}</span></div>
  <div v-if="db.confirmOpen" class="confirmation"><strong>确认执行本数据源上的 SQL</strong><p>{{sourceLabel}}；允许写入不表示绕过生产保护。</p><pre>{{db.pendingSql}}</pre><button :disabled="db.busy" @click="db.confirmRun()">确认执行</button><button @click="db.cancelConfirm()">取消</button></div>
  <section v-if="historyOpen" class="history"><label><input type="checkbox" :checked="db.rememberQueries" @change="db.setRememberQueries(($event.target as HTMLInputElement).checked)"/>仅在本次会话记录 SQL（最多 30 条，不落盘）</label><button @click="db.clearQueryHistory()">清空历史</button><button v-for="item in db.queryHistory" :key="item.id" :disabled="db.busy" :title="item.sql" @click="db.reuseHistory(item)">{{item.at}} · {{item.connection}} · {{item.sql.slice(0,120)}}</button></section>
  <p v-if="db.error" class="error" role="alert">{{db.error}}</p><p v-if="notice" role="status">{{notice}}</p>
  <nav v-if="db.document.batchResults?.length>1" class="actions" aria-label="脚本结果"><button v-for="(_,index) in db.document.batchResults" :key="index" @click="selectResult(index)">语句 {{index+1}} 的结果</button></nav>
  <section v-if="view && !db.document.tableData" class="results"><header class="actions"><strong>{{view.rowLabel}} · {{view.elapsedLabel}}</strong><input v-model="filter" placeholder="筛选已读取结果" aria-label="筛选已读取结果"/><select v-model="exportFormat" aria-label="导出格式"><option value="csv">CSV</option><option value="tsv">TSV</option><option value="json">JSON</option></select><button @click="copy">复制</button><button @click="download">导出当前结果</button></header><p class="source-note">这里只排序、筛选已读取的 {{view.rows.length}} 行，不会假装在服务器查询全部数据；导出同样受读取上限限制。</p><p v-for="warning in view.warnings" :key="warning" class="warning">{{warning}}</p><div v-if="db.document.tablePreview" class="actions"><span>表数据服务端批次 {{Math.floor(db.document.tablePreview.offset/100)+1}}</span><button :disabled="db.busy||db.document.tablePreview.offset===0" @click="db.previousTablePage()">上一批 100 行</button><button :disabled="db.busy||view.rowCount<100||db.document.tablePreview.offset>=50000" @click="db.nextTablePage()">下一批 100 行</button></div><div class="grid"><table><thead><tr><th v-for="(column,index) in view.columns" :key="index"><button @click="sort(index)">{{column}}{{sortColumn===index?(ascending?' ↑':' ↓'):''}}</button></th></tr></thead><tbody><tr v-for="(row,index) in displayed" :key="index"><td v-for="(cell,col) in row" :key="col" :title="cell">{{cell}}</td></tr></tbody></table></div><footer class="actions"><button :disabled="page===0" @click="page--">上一显示页</button><span>{{page+1}} / {{Math.max(1,Math.ceil(rows.length/50))}}</span><button :disabled="(page+1)*50>=rows.length" @click="page++">下一显示页</button></footer></section>
</section>
</template>
<style scoped>
.database-workspace{display:flex;flex-direction:column;min-width:0;min-height:0;gap:7px;flex:1;overflow:auto;padding:8px;background:#fff;color:#293c50;font-size:12px}.documents{display:flex;overflow-x:auto;gap:4px;min-height:34px}.document-tab{display:flex;white-space:nowrap;border-bottom:2px solid transparent}.document-tab.active{border-bottom-color:#3477bc;background:#edf4fc}.document-tab button{border:0;background:transparent}.session,.actions{display:flex;align-items:center;flex-wrap:wrap;gap:7px}.session strong{margin-right:auto}.source-note{font-size:11px;color:#65798c;margin:3px 0}.confirmation{padding:10px;border:1px solid #d7b56a;background:#fff8e8;border-radius:5px}.confirmation pre{max-height:140px;overflow:auto;white-space:pre-wrap}.confirmation label{margin-right:10px}.results{border:1px solid #d3deea;border-radius:6px;padding:7px}.grid{overflow:auto;max-height:440px;min-height:70px}table{border-collapse:collapse;width:max-content;min-width:100%;font:12px/1.6 ui-monospace,monospace}th,td{border:1px solid #e0e7ef;padding:5px 9px;max-width:420px;white-space:pre;overflow:hidden;text-overflow:ellipsis}th{position:sticky;top:0;background:#f2f6fb;text-align:left;z-index:1}th button{background:transparent;border:0;padding:0}.history{display:flex;flex-direction:column;gap:5px;background:#f4f8fc;padding:8px}.history button{text-align:left}.error{color:#982828;background:#fff0f0;padding:8px}.warning{color:#846222}button,input,select{font:inherit;border:1px solid #c5d3e2;border-radius:4px;padding:5px 8px;color:inherit;background:#fff}button{cursor:pointer}button:disabled{opacity:.45;cursor:not-allowed}p{overflow-wrap:anywhere}
</style>
