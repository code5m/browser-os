<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useDatabaseStore } from '../state/useDatabaseStore';
import { buildResultView, decodeDbValue, riskLabel, productionLabel } from '../../../utils/dbUi';
import { exportRows } from '../model/studioSql.mjs';
import SqlEditor from './SqlEditor.vue';
import TableDataEditor from './TableDataEditor.vue';
const db=useDatabaseStore();
const workspaceTab=ref<'query'|'schema'|'assistant'>('query');
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
  <nav class="workspace-tabs" role="tablist" aria-label="数据库工作区">
    <button role="tab" :aria-selected="workspaceTab==='query'" :class="{active:workspaceTab==='query'}" @click="workspaceTab='query'">◉ Query</button>
    <button role="tab" :aria-selected="workspaceTab==='schema'" :class="{active:workspaceTab==='schema'}" @click="workspaceTab='schema'">▦ Schema</button>
    <button role="tab" :aria-selected="workspaceTab==='assistant'" :class="{active:workspaceTab==='assistant'}" @click="workspaceTab='assistant'">✦ AI Assistant</button>
  </nav>
  <template v-if="workspaceTab==='query'">
  <div v-if="db.closePending" class="confirmation">关闭会丢弃本页 SQL 草稿，并回滚本页未提交事务。<button @click="db.closeDocument(db.closePending!,true)">回滚并关闭页签</button><button @click="db.closePending=null">保留页签</button></div>
  <header class="session"><strong>{{sourceLabel}}</strong><span>{{db.document.sessionId?'独立原生会话':'按需连接'}} · 事务：{{db.document.transaction||'idle'}}</span><button :disabled="db.busy||!db.document.connId||db.document.transaction!=='idle'" @click="db.sessionAction('begin',{writable:false,confirmed:false})">只读事务</button><button :disabled="db.busy||!db.document.connId||db.document.transaction!=='idle'" @click="transaction('begin',true)">写事务…</button><button :disabled="db.busy||!db.document.sessionId||db.document.transaction==='idle'||['failed','unknown'].includes(db.document.transaction)" @click="transaction('commit')">提交事务…</button><button :disabled="db.busy||!db.document.sessionId||db.document.transaction==='idle'" @click="transaction('rollback')">回滚事务…</button><button :disabled="!db.document.sessionId" @click="closeSession">关闭会话</button></header>
  <p class="source-note">每页 SQL 固定绑定自己的数据源；选择另一个数据源不会把已有 SQL 静默改到新库。最多 4 个原生会话，空闲 5 分钟自动回滚并释放。</p>
  <div v-if="pendingTransaction" class="confirmation"><strong>{{pendingTransaction.action==='commit'?'确认提交本会话的事务':pendingTransaction.action==='rollback'?'确认回滚本会话的未提交修改':'确认开启允许写入的事务'}}</strong><p>{{sourceLabel}}；生产环境及未知环境仍由原生安全策略拒绝写入。</p><label><input v-model="acknowledged" type="checkbox"/>我已核对当前页的数据源与事务</label><button :disabled="!acknowledged||db.busy" @click="confirmTransaction">确认</button><button @click="pendingTransaction=null">取消</button></div>
  <TableDataEditor v-if="db.document.tableData"/>
  <SqlEditor v-else :model-value="db.sql" :disabled="db.document.busy||db.confirmOpen||db.document.closing||db.document.closeFailed||db.document.transaction==='unknown'" :objects="db.schema.map(item=>item.name)" :columns="columns" :catalog="db.completionCatalog" :source-label="sourceLabel" @update:model-value="db.sql=$event" @run="run"/>
  <div class="actions"><button :disabled="!db.document.busy" @click="db.cancelQuery()">取消查询</button><button :disabled="db.busy||!db.sql.trim()||config?.kind!=='mysql'" @click="db.explainCurrentSql()">EXPLAIN SELECT</button><button :disabled="!raw&&!db.error" @click="db.clearResult()">清空结果</button><button :aria-pressed="historyOpen" @click="historyOpen=!historyOpen">查询历史</button><span>{{riskLabel(db.risk)}} · {{productionLabel(db.verdict)}}</span></div>
  <div v-if="db.confirmOpen" class="confirmation"><strong>确认执行本数据源上的 SQL</strong><p>{{sourceLabel}}；允许写入不表示绕过生产保护。</p><pre>{{db.pendingSql}}</pre><button :disabled="db.busy" @click="db.confirmRun()">确认执行</button><button @click="db.cancelConfirm()">取消</button></div>
  <section v-if="historyOpen" class="history"><label><input type="checkbox" :checked="db.rememberQueries" @change="db.setRememberQueries(($event.target as HTMLInputElement).checked)"/>仅在本次会话记录 SQL（最多 30 条，不落盘）</label><button @click="db.clearQueryHistory()">清空历史</button><button v-for="item in db.queryHistory" :key="item.id" :disabled="db.busy" :title="item.sql" @click="db.reuseHistory(item)">{{item.at}} · {{item.connection}} · {{item.sql.slice(0,120)}}</button></section>
  <p v-if="db.error" class="error" role="alert">{{db.error}}</p><p v-if="notice" role="status">{{notice}}</p>
  <nav v-if="db.document.batchResults?.length>1" class="actions" aria-label="脚本结果"><button v-for="(_,index) in db.document.batchResults" :key="index" @click="selectResult(index)">语句 {{index+1}} 的结果</button></nav>
  <section v-if="view && !db.document.tableData" class="results"><header class="actions"><strong>{{view.rowLabel}} · {{view.elapsedLabel}}</strong><input v-model="filter" placeholder="筛选已读取结果" aria-label="筛选已读取结果"/><select v-model="exportFormat" aria-label="导出格式"><option value="csv">CSV</option><option value="tsv">TSV</option><option value="json">JSON</option></select><button @click="copy">复制</button><button @click="download">导出当前结果</button></header><p class="source-note">这里只排序、筛选已读取的 {{view.rows.length}} 行，不会假装在服务器查询全部数据；导出同样受读取上限限制。</p><p v-for="warning in view.warnings" :key="warning" class="warning">{{warning}}</p><div v-if="db.document.tablePreview" class="actions"><span>表数据服务端批次 {{Math.floor(db.document.tablePreview.offset/100)+1}}</span><button :disabled="db.busy||db.document.tablePreview.offset===0" @click="db.previousTablePage()">上一批 100 行</button><button :disabled="db.busy||view.rowCount<100||db.document.tablePreview.offset>=50000" @click="db.nextTablePage()">下一批 100 行</button></div><div class="grid"><table><thead><tr><th v-for="(column,index) in view.columns" :key="index"><button @click="sort(index)">{{column}}{{sortColumn===index?(ascending?' ↑':' ↓'):''}}</button></th></tr></thead><tbody><tr v-for="(row,index) in displayed" :key="index"><td v-for="(cell,col) in row" :key="col" :title="cell">{{cell}}</td></tr></tbody></table></div><footer class="actions"><button :disabled="page===0" @click="page--">上一显示页</button><span>{{page+1}} / {{Math.max(1,Math.ceil(rows.length/50))}}</span><button :disabled="(page+1)*50>=rows.length" @click="page++">下一显示页</button></footer></section>
  <div class="dbx-bottom" aria-label="查询历史与运行状态">
    <section class="dbx-bottom-pane"><header class="pane-head"><span>查询历史</span><button class="small-btn" @click="db.clearQueryHistory()">清除</button></header><div v-if="!db.queryHistory.length" class="bottom-empty">暂无本次会话查询</div><button v-for="item in db.queryHistory.slice(0,6)" :key="'bottom-'+item.id" class="history-row" :disabled="db.busy" :title="item.sql" @click="db.reuseHistory(item)"><span>{{item.connection}} · {{item.sql.slice(0,80)}}</span><span class="muted">{{item.elapsed}}</span></button></section>
    <section class="dbx-bottom-pane"><header class="pane-head"><span>任务 / 会话</span><span class="muted">{{db.backendReady?'native bridge':'backend unavailable'}}</span></header><div class="list-row"><span><i class="status-dot" :class="{off:!db.backendReady}"></i> 数据库命令</span><span class="muted">{{db.backendReady?'ready':'waiting'}}</span></div><div class="list-row"><span>当前会话</span><span class="muted">{{db.document.sessionId?'独立会话':'按需连接'}}</span></div><div class="list-row"><span>结果范围</span><span class="muted">{{view?view.rows.length:0}} 行</span></div></section>
  </div>
  </template>
  <section v-else-if="workspaceTab==='schema'" class="workspace-placeholder" aria-label="Schema 浏览器">
    <header><strong>Schema Browser</strong><span class="muted">从左侧对象资源树选择表或视图</span></header>
    <div class="schema-summary"><div><b>{{db.schema.length}}</b><span>对象</span></div><div><b>{{db.selectedObject||'—'}}</b><span>当前对象</span></div><div><b>{{db.objectDetailRows.length}}</b><span>当前详情记录</span></div></div>
    <p v-if="!db.activeId" class="hint">请先连接数据库；连接成功后这里展示真实 Schema 元数据。</p>
    <p v-else class="hint">左侧单击对象查看字段、索引、外键和 DDL；双击对象直接读取前 100 行。</p>
  </section>
  <section v-else class="workspace-placeholder assistant-workspace" aria-label="Database Assistant">
    <header><strong>Database Assistant</strong><span class="muted">仅基于当前会话，不会绕过数据库安全门禁</span></header>
    <p>当前 SQL：<code>{{db.sql.trim()||'尚未输入查询'}}</code></p>
    <div class="assistant-suggestions"><button @click="workspaceTab='query'">回到 Query 编辑器</button><button @click="db.sql=db.sql.trim() ? db.sql : 'SELECT 1;' ; workspaceTab='query'">生成安全探针 SELECT 1</button><button @click="db.selectedObject&& (workspaceTab='schema')" :disabled="!db.selectedObject">解释当前对象结构</button></div>
    <p class="hint">AI Assistant 当前提供本地、可审计的辅助入口；真实 SQL 仍必须回到 Query 编辑器并通过原生安全策略执行。</p>
  </section>
</section>
</template>
<style scoped>
.database-workspace{display:flex;flex-direction:column;min-width:0;min-height:0;gap:7px;flex:1;overflow:auto;padding:8px;background:#fff;color:#293c50;font-size:12px}.documents{display:flex;overflow-x:auto;gap:4px;min-height:34px}.document-tab{display:flex;white-space:nowrap;border-bottom:2px solid transparent}.document-tab.active{border-bottom-color:#3477bc;background:#edf4fc}.document-tab button{border:0;background:transparent}.workspace-tabs{display:flex;gap:4px;align-items:center;border-bottom:1px solid #2a3546;min-height:34px}.workspace-tabs button{border:0;border-bottom:2px solid transparent;background:transparent;color:#8794a8;border-radius:0}.workspace-tabs button.active{border-bottom-color:#5aa9ff;color:#fff;background:#1d293b}.session,.actions{display:flex;align-items:center;flex-wrap:wrap;gap:7px}.session strong{margin-right:auto}.source-note{font-size:11px;color:#65798c;margin:3px 0}.confirmation{padding:10px;border:1px solid #d7b56a;background:#fff8e8;border-radius:5px}.confirmation pre{max-height:140px;overflow:auto;white-space:pre-wrap}.confirmation label{margin-right:10px}.results{border:1px solid #d3deea;border-radius:6px;padding:7px}.grid{overflow:auto;max-height:440px;min-height:70px}table{border-collapse:collapse;width:max-content;min-width:100%;font:12px/1.6 ui-monospace,monospace}th,td{border:1px solid #e0e7ef;padding:5px 9px;max-width:420px;white-space:pre;overflow:hidden;text-overflow:ellipsis}th{position:sticky;top:0;background:#f2f6fb;text-align:left;z-index:1}th button{background:transparent;border:0;padding:0}.history{display:flex;flex-direction:column;gap:5px;background:#f4f8fc;padding:8px}.history button{text-align:left}.error{color:#982828;background:#fff0f0;padding:8px}.warning{color:#846222}button,input,select{font:inherit;border:1px solid #c5d3e2;border-radius:4px;padding:5px 8px;color:inherit;background:#fff}button{cursor:pointer}button:disabled{opacity:.45;cursor:not-allowed}p{overflow-wrap:anywhere}.workspace-placeholder{display:flex;flex-direction:column;gap:14px;min-height:330px;padding:28px 34px;background:#151c29;border:1px solid #2a3546;color:#d8dee9}.workspace-placeholder header{display:flex;justify-content:space-between;align-items:center;font-size:14px}.schema-summary{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:10px}.schema-summary div{display:flex;flex-direction:column;gap:6px;padding:14px;background:#1b2636;border:1px solid #2a3546;border-radius:7px}.schema-summary b{font-size:18px;color:#a9d4ff}.schema-summary span{color:#8794a8}.assistant-workspace code{display:block;padding:12px;background:#101722;border:1px solid #2a3546;white-space:pre-wrap}.assistant-suggestions{display:flex;gap:8px;flex-wrap:wrap}
.dbx-bottom{display:grid;grid-template-columns:1fr 1fr;min-height:150px;background:#151c29;border:1px solid #2a3546}.dbx-bottom-pane{min-width:0;overflow:auto;border-right:1px solid #2a3546}.dbx-bottom-pane:last-child{border-right:0}.dbx-bottom .pane-head{height:34px;padding:0 10px;color:#aebbd0;border-bottom:1px solid #2a3546}.dbx-bottom .small-btn{padding:4px 7px;background:transparent}.history-row,.dbx-bottom .list-row{display:flex;width:100%;justify-content:space-between;gap:10px;text-align:left;padding:8px 10px;border:0;border-bottom:1px solid #263143;border-radius:0;background:transparent;color:#bdc8d7;font-size:11px}.history-row:hover{background:#1d293b}.history-row span:first-child{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.bottom-empty{padding:20px 10px;color:#8794a8;font-size:11px}.muted{color:#8794a8}.status-dot{display:inline-block;width:7px;height:7px;margin-right:6px;border-radius:50%;background:#46d18d}.status-dot.off{background:#f4b860}
</style>
<style scoped>
.database-workspace :deep(.highlight),.database-workspace :deep(.editor-surface textarea){font:14px/1.75 "JetBrains Mono","DejaVu Sans Mono",Consolas,monospace;font-weight:400;padding:14px 16px}
.database-workspace :deep(.sql-editor .highlight){color:#dce3ed}
.database-workspace :deep(.keyword){color:#c4a7ff;font-weight:400}
.database-workspace :deep(.string){color:#a4cf8c}
.database-workspace :deep(.number){color:#89c8ec}
.database-workspace :deep(.comment){color:#8d9bad;font-style:normal}
.database-workspace :deep(.completions){position:absolute;z-index:1;display:flex;flex-direction:column;flex-wrap:nowrap;width:min(290px,100%);max-height:150px;padding:4px;box-sizing:border-box;border:1px solid #46556c;border-radius:6px;background:#202b3b;box-shadow:0 6px 18px #0005}
.database-workspace :deep(.completions button){display:flex;align-items:center;justify-content:space-between;gap:16px;flex:none;min-height:29px;border:0;background:transparent;text-align:left;font:13px/1.5 "DejaVu Sans Mono",monospace;color:#e1e8f2}
.database-workspace :deep(.completions button[aria-selected=true]){background:#345176;color:#fff}
.database-workspace :deep(.completions small){font:11px system-ui,sans-serif;color:#b4c3d7}
</style>
<style scoped>
/* DBX central workbench surfaces. These remain real query/result states, not fixtures. */
.database-workspace{background:#111722;color:#d8dee9;padding:10px 14px;gap:8px}
.document-tab.active{border-bottom-color:#5aa9ff;background:#1d293b}.document-tab button{color:#bdc8d7}.document-tab.active button{color:#fff}
.session{background:#151c29;border:1px solid #2a3546;border-radius:7px;padding:8px}.session span,.source-note{color:#8794a8}.session button,.actions button,.actions input,.actions select{background:#182131;border-color:#2a3546;color:#cbd6e4}.session button:hover,.actions button:hover{background:#243247;color:#fff}
.results{background:#151c29;border-color:#2a3546}.results th{background:#1a2433;color:#91a6c0;border-color:#263143}.results td{color:#d2dbea;border-color:#263143}.results th button{color:#a9d4ff}.results tr:hover td{background:#1b2a3e}.confirmation{background:#3a2a14;border-color:#6c4d1d;color:#f4c77a}.error{background:#3a1e25;color:#fa7777}.warning{color:#f4b860}
.database-workspace :deep(.sql-editor),.database-workspace :deep(.table-data-editor){background:#151c29;border-color:#2a3546;color:#d8dee9}.database-workspace :deep(.editor-tools),.database-workspace :deep(.find),.database-workspace :deep(.diagnostics){background:#151c29;border-color:#2a3546}.database-workspace :deep(.editor-surface){border-color:#2a3546;background:#101722}.database-workspace :deep(.highlight){color:#bdc8d7}.database-workspace :deep(.editor-surface textarea){color:transparent;background:transparent;caret-color:#fff}.database-workspace :deep(.hint){color:#8794a8}.database-workspace :deep(button),.database-workspace :deep(input),.database-workspace :deep(select),.database-workspace :deep(textarea){background:#182131;border-color:#2a3546;color:#cbd6e4}
</style>
