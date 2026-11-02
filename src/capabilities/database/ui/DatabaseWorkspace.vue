<script setup lang="ts">
import { computed, defineAsyncComponent, ref, watch } from 'vue';
const AiAssistant=defineAsyncComponent(()=>import('../../../components/system/AiAssistant.vue'));
import { useDatabaseStore } from '../state/useDatabaseStore';
import { riskLabel, productionLabel } from '../../../utils/dbUi';
import SqlEditor from './SqlEditor.vue';
import TableDataEditor from './TableDataEditor.vue';
import DatabaseOutput from './DatabaseOutput.vue';
const db=useDatabaseStore();
const workspaceTab=ref<'query'|'schema'|'assistant'>('query');
const historyOpen=ref(false);
const resultMaximized=ref(false);
const pendingTransaction=ref<{document:string;action:string;writable?:boolean}|null>(null),acknowledged=ref(false),notice=ref('');
const connection=computed(()=>db.connections.find(c=>c.id===db.document.connId));
const config=computed(()=>db.configs.find(c=>c.id===db.document.connId));
const sourceLabel=computed(()=>connection.value?`${connection.value.name} · ${config.value?.database||''}`:'请从左侧选择数据源');
const columns=computed(()=>db.objectDetailMode==='columns'?db.objectDetailRows.map(row=>row[0]):[]);
watch(()=>db.activeDocument,()=>{pendingTransaction.value=null;acknowledged.value=false;});
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
  <header v-show="!resultMaximized" class="session"><strong>{{sourceLabel}}</strong><span>{{db.document.sessionId?'独立原生会话':'按需连接'}} · 事务：{{db.document.transaction||'idle'}}</span><button :disabled="db.busy||!db.document.connId||db.document.transaction!=='idle'" @click="db.sessionAction('begin',{writable:false,confirmed:false})">只读事务</button><button :disabled="db.busy||!db.document.connId||db.document.transaction!=='idle'" @click="transaction('begin',true)">写事务…</button><button :disabled="db.busy||!db.document.sessionId||db.document.transaction==='idle'||['failed','unknown'].includes(db.document.transaction)" @click="transaction('commit')">提交事务…</button><button :disabled="db.busy||!db.document.sessionId||db.document.transaction==='idle'" @click="transaction('rollback')">回滚事务…</button><button :disabled="!db.document.sessionId" @click="closeSession">关闭会话</button></header>
  <p v-show="!resultMaximized" class="source-note">每页 SQL 固定绑定自己的数据源；选择另一个数据源不会把已有 SQL 静默改到新库。最多 4 个原生会话，空闲 5 分钟自动回滚并释放。</p>
  <div v-if="pendingTransaction && !resultMaximized" class="confirmation"><strong>{{pendingTransaction.action==='commit'?'确认提交本会话的事务':pendingTransaction.action==='rollback'?'确认回滚本会话的未提交修改':'确认开启允许写入的事务'}}</strong><p>{{sourceLabel}}；生产环境及未知环境仍由原生安全策略拒绝写入。</p><label><input v-model="acknowledged" type="checkbox"/>我已核对当前页的数据源与事务</label><button :disabled="!acknowledged||db.busy" @click="confirmTransaction">确认</button><button @click="pendingTransaction=null">取消</button></div>
  <TableDataEditor v-if="db.document.tableData"/>
  <SqlEditor v-else v-show="!resultMaximized" :model-value="db.sql" :disabled="db.document.busy||db.confirmOpen||db.document.closing||db.document.closeFailed||db.document.transaction==='unknown'" :objects="db.schema.map(item=>item.name)" :columns="columns" :catalog="db.completionCatalog" :source-label="sourceLabel" @update:model-value="db.sql=$event" @run="run"/>
  <div v-show="!resultMaximized" class="actions"><button :disabled="!db.document.busy" @click="db.cancelQuery()">取消查询</button><button :disabled="db.busy||!db.sql.trim()||config?.kind!=='mysql'" @click="db.explainCurrentSql()">EXPLAIN SELECT</button><button :disabled="!raw&&!db.error" @click="db.clearResult()">清空结果</button><button :aria-pressed="historyOpen" @click="historyOpen=!historyOpen">查询历史</button><span>{{riskLabel(db.risk)}} · {{productionLabel(db.verdict)}}</span></div>
  <div v-if="db.confirmOpen && !resultMaximized" class="confirmation"><strong>确认执行本数据源上的 SQL</strong><p>{{sourceLabel}}；允许写入不表示绕过生产保护。</p><pre>{{db.pendingSql}}</pre><button :disabled="db.busy" @click="db.confirmRun()">确认执行</button><button @click="db.cancelConfirm()">取消</button></div>
  <section v-if="historyOpen && !resultMaximized" class="history"><label><input type="checkbox" :checked="db.rememberQueries" @change="db.setRememberQueries(($event.target as HTMLInputElement).checked)"/>仅在本次会话记录 SQL（最多 30 条，不落盘）</label><button @click="db.clearQueryHistory()">清空历史</button><button v-for="item in db.queryHistory" :key="item.id" :disabled="db.busy" :title="item.sql" @click="db.reuseHistory(item)">{{item.at}} · {{item.connection}} · {{item.sql.slice(0,120)}}</button></section>
  <p v-if="db.error && !resultMaximized" class="error" role="alert">{{db.error}}</p><p v-if="notice" role="status">{{notice}}</p>
  <nav v-if="db.document.batchResults?.length>1" class="actions" aria-label="脚本结果"><button v-for="(_,index) in db.document.batchResults" :key="index" @click="selectResult(index)">语句 {{index+1}} 的结果</button></nav>
  <DatabaseOutput v-if="!db.document.tableData" @maximize="resultMaximized=$event"/>
  </template>
  <section v-else-if="workspaceTab==='schema'" class="workspace-placeholder" aria-label="Schema 浏览器">
    <header><strong>Schema Browser</strong><span class="muted">从左侧对象资源树选择表或视图</span></header>
    <div class="schema-summary"><div><b>{{db.schema.length}}</b><span>对象</span></div><div><b>{{db.selectedObject||'—'}}</b><span>当前对象</span></div><div><b>{{db.objectDetailRows.length}}</b><span>当前详情记录</span></div></div>
    <p v-if="!db.activeId" class="hint">请先连接数据库；连接成功后这里展示真实 Schema 元数据。</p>
    <p v-else class="hint">左侧单击对象查看字段、索引、外键和 DDL；双击对象直接读取前 100 行。</p>
  </section>
  <section v-else class="workspace-placeholder assistant-workspace" aria-label="Database Assistant">
    <header><strong>Database Assistant</strong><span class="muted">仅基于当前会话，不会绕过数据库安全门禁</span></header>
    <AiAssistant scope="database" :context="db.sql"/>
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

<style scoped>
/* Query editor grows above an independent Output dock; do not scroll the whole studio. */
.database-workspace{display:flex;flex-direction:column;height:100%;min-height:0;min-width:0;overflow:hidden!important;gap:5px;padding:7px 9px!important}
.database-workspace :deep(.sql-editor){flex:1 1 0;min-height:125px;overflow:hidden;border-radius:3px}
.database-workspace :deep(.sql-editor .editor-surface){flex:1 1 0;min-height:0;overflow:auto}
.database-workspace :deep(.table-data-editor){flex:1;min-height:0;overflow:auto}
.database-workspace :deep(.editor-surface textarea){min-height:100%!important}
.database-workspace :deep(.ide-result-pane){border-radius:0}
.database-workspace :deep(.ide-output-head button){min-width:40px}
.database-workspace .documents{flex:0 0 30px;min-height:30px;margin-bottom:0}
.database-workspace .workspace-tabs{flex:0 0 29px;min-height:29px}
.database-workspace .session{flex-shrink:0;padding:4px 7px;gap:4px;font-size:11px}
.database-workspace .session button{padding:3px 6px}
.database-workspace .source-note{flex-shrink:0;margin:0}
.database-workspace .actions{flex-shrink:0}
@media(max-height:660px){.database-workspace .source-note{display:none}}
</style>
