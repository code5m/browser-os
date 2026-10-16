"""Bind each SQL document to its own source, session, transaction and history."""
import re

path = 'src/capabilities/database/state/useDatabaseStore.ts'
s = read(path)

def function(text, name, replacement):
    match = re.search(r'^  (?:async )?function ' + re.escape(name) + r'\(', text, re.M)
    if not match:
        raise RuntimeError('DATABASE_FUNCTION_MISSING: ' + name)
    tail = re.search(r'^  (?:async )?function \w+\(', text[match.end():], re.M)
    if not tail:
        raise RuntimeError('DATABASE_FUNCTION_END_MISSING: ' + name)
    end = match.end() + tail.start()
    return text[:match.start()] + replacement.rstrip() + '\n\n' + text[end:]

if '// DEVELOPER_STUDIO_SOURCE_BOUND_DOCUMENTS' not in s:
    s = once(s, 'import { computed, ref } from "vue";', 'import { computed, ref, watch } from "vue";\nimport { sqlStatements } from "../model/studioSql.mjs";')
    s = once(s, 'tablePreview:{name:string;offset:number}|null };', 'tablePreview:{name:string;offset:number}|null; sessionId:string|null; transaction:string; batchResults:DbQueryResult[]; pendingBatch:boolean; closing:boolean; risk:DbRiskLevel; verdict:ProductionVerdict };')
    s = once(s, "pending:null, tablePreview:null });", "pending:null, tablePreview:null, sessionId:null, transaction:'idle', batchResults:[], pendingBatch:false, closing:false, risk:'unknown', verdict:'Unknown' });")
    s = once(s, 'const queryHistory = ref<{id:string;connection:string;sql:string;at:string;elapsed:string}[]>([]);', 'const queryHistory = ref<{id:string;connId:string;connection:string;sql:string;at:string;elapsed:string}[]>([]);')
    s = once(s, 'const risk = ref<DbRiskLevel>("unknown");', 'const risk = computed({get:()=>document.value.risk,set:(value:DbRiskLevel)=>{document.value.risk=value;}});')
    s = once(s, 'const verdict = ref<ProductionVerdict>("Unknown");', 'const verdict = computed({get:()=>document.value.verdict,set:(value:ProductionVerdict)=>{document.value.verdict=value;}});')
    s = once(s, '    const queryIds = documents.value.map((doc) => doc.queryId)', '    await Promise.all(documents.value.map(doc => releaseSession(doc)));\n    const queryIds = documents.value.map((doc) => doc.queryId)')
    s = function(s, 'selectConnection', '''  function selectConnection(c: DbConnectionSummary) {
    if (document.value.busy || document.value.pending || document.value.closing) return;
    const cfg = configs.value.find(it => it.id === c.id && it.enabled);
    if (!cfg) return;
    const previous = document.value;
    // Never silently retarget existing SQL, results or a transaction.
    if (previous.connId !== c.id && (previous.sql.trim() || previous.raw || previous.sessionId)) {
      const reusable = documents.value.find(d => d.connId === c.id && !d.busy && !d.pending);
      if (reusable) activeDocument.value = reusable.id;
      else {
        if (documents.value.length >= 12) { previous.error = '最多打开 12 个查询页签'; return; }
        const next = makeDoc(++sequence, c.id); documents.value.push(next); activeDocument.value = next.id;
      }
    } else document.value.connId = c.id;
    synchronizeDocumentSource();
  }''')
    s = function(s, 'closeDocument', '''  async function closeDocument(id:string, confirmed=false) {
    const doc = documents.value.find(d => d.id === id);
    if (!doc || doc.busy || doc.closing) return;
    if ((doc.sql.trim() || doc.transaction !== 'idle') && !confirmed) { closePending.value = id; return; }
    closePending.value = null;
    if (!await releaseSession(doc)) return;
    documents.value = documents.value.filter(d => d.id !== id);
    if (!documents.value.length) documents.value.push(makeDoc(++sequence,null));
    if (activeDocument.value === id) activeDocument.value = documents.value[0].id;
  }''')
    s = function(s, 'cancelQuery', '''  async function cancelQuery() {
    const doc = document.value;
    try {
      if (doc.sessionId) {
        const response = await bridge.dbStudioCancel(doc.sessionId);
        if (response.cancel_requested) doc.error = '已请求取消；等待数据库停止后才会释放事务。';
        else if (!response.completed) doc.error = '查询正在启动，正在关闭该会话以确保停止。';
        if (!response.cancel_requested && !response.completed) void releaseSession(doc);
      } else if (doc.queryId && !await bridge.dbCancel(doc.queryId)) doc.error = '查询已完成或尚未开始';
    } catch(e) { doc.error = describeError(e); }
  }''')
    s = function(s, 'reuseHistory', '''  function reuseHistory(item: {sql:string;connection:string;connId?:string}) {
    if (!item.connId || !configs.value.some(c=>c.id===item.connId && c.enabled)) {
      error.value = '历史数据源已删除或不可用，不能将 SQL 静默切换到当前库。'; return;
    }
    if (documents.value.length >= 12) { error.value = '最多打开 12 个查询页签'; return; }
    const next=makeDoc(++sequence,item.connId); next.name='历史 SQL'; next.sql=item.sql;
    documents.value.push(next); activeDocument.value=next.id;
  }''')
    s = function(s, 'requestRun', '''  function requestRun() { void requestRunText(sql.value, false); }''')
    s = function(s, 'cancelConfirm', '''  function cancelConfirm() { document.value.pending = null; document.value.pendingBatch = false; }''')
    s = function(s, 'confirmRun', '''  function confirmRun() {
    if (!lifecycleActive()) return;
    const doc=document.value, text=doc.pending, batch=doc.pendingBatch;
    doc.pending=null; doc.pendingBatch=false;
    if(text) void execute(text,true,batch);
  }''')
    s = function(s, 'execute', '''  async function execute(text:string, confirmed:boolean, batch=false) {
    if (!lifecycleActive()) return;
    const lifecycle=databaseLifecycleSnapshot(), doc=document.value, connId=doc.connId;
    if (!backendReady.value || !connId || doc.busy || doc.closing) return;
    doc.busy=true; doc.error=''; doc.queryId=crypto.randomUUID();
    try {
      let result:DbQueryResult;
      if (typeof bridge.dbStudioOpen === 'function') {
        const sessionId=await ensureSession(doc);
        const reply=await bridge.dbStudioCall(sessionId, batch
          ? {action:'batch',statements:sqlStatements(text).map((s:{text:string})=>s.text),confirmed,timeout_secs:30}
          : {action:'execute',sql:text,confirmed,timeout_secs:30});
        doc.transaction=reply.transaction || doc.transaction;
        doc.batchResults=reply.results || (reply.result ? [reply.result] : []);
        result=(reply.result || reply.results?.at(-1)) as DbQueryResult;
        if(!result) throw new Error('数据库未返回可验证的查询结果');
      } else {
        // Compatibility for the existing simple-query bridge and its tests;
        // a batch or transaction never degrades to unrelated connections.
        if(batch) throw new Error('当前运行版本未提供数据库会话，请重新安装');
        result=await bridge.dbQuery({conn_id:connId,sql:text,timeout_secs:null,confirm_write:confirmed,query_id:doc.queryId});
      }
      const current=databaseLifecycleSnapshot();
      if(!current.active || current.generation!==lifecycle.generation || doc.connId!==connId || !documents.value.some(d=>d.id===doc.id)) return;
      const next=buildResultView(result); doc.raw=result; doc.result=next; doc.risk=next.risk; doc.verdict=next.verdict;
      if(rememberQueries.value && (result as any).state!=='failed') {
        const connection=connections.value.find(c=>c.id===connId)?.name || '数据库';
        queryHistory.value.unshift({id:crypto.randomUUID(),connId,connection,sql:text,at:new Date().toLocaleTimeString(),elapsed:next.elapsedLabel});
        queryHistory.value=queryHistory.value.slice(0,30);
      }
    } catch(e) {
      doc.error=describeError(e);
      if(doc.sessionId && !doc.closing) {
        try { const state=await bridge.dbStudioCall(doc.sessionId,{action:'status'}); doc.transaction=state.transaction; }
        catch { doc.transaction='unknown'; doc.error+='；会话状态未知，请关闭会话，不要重复执行写入。'; }
      }
    } finally { doc.busy=false; doc.queryId=null; }
  }''')
    helpers = '''
  // DEVELOPER_STUDIO_SOURCE_BOUND_DOCUMENTS
  const openingSessions = new Map<string, Promise<string>>();
  function synchronizeDocumentSource() {
    const cfg=configs.value.find(c=>c.id===document.value.connId);
    ++schemaGeneration; ++columnsGeneration; schema.value=[]; selectedObject.value=''; objectDetailRows.value=[];
    if(!cfg) { resetForm(); return; }
    form.value={...emptyConnectionForm(),id:cfg.id,name:cfg.name,kind:cfg.kind,allowWrite:cfg.allow_write,enabled:cfg.enabled,
      database:cfg.database,host:cfg.host||'',portText:cfg.port?String(cfg.port):'',username:cfg.username||'',sslMode:cfg.ssl_mode,
      productionHint:cfg.production_hint===true?'yes':cfg.production_hint===false?'no':''};
    void refreshSchema();
  }
  function activateDocument(id:string) { if(documents.value.some(d=>d.id===id)) activeDocument.value=id; }
  watch(activeDocument,synchronizeDocumentSource);
  async function ensureSession(doc:QueryDocument):Promise<string> {
    if(doc.closing) throw new Error('数据库会话正在关闭');
    if(doc.sessionId) return doc.sessionId;
    const existing=openingSessions.get(doc.id); if(existing) return existing;
    const connId=doc.connId,lifecycle=databaseLifecycleSnapshot();
    if(!connId) throw new Error('请先选择数据源');
    const promise=(async()=>{
      const opened=await bridge.dbStudioOpen(connId);
      const current=databaseLifecycleSnapshot();
      if(doc.closing||!current.active||current.generation!==lifecycle.generation||doc.connId!==connId||!documents.value.some(d=>d.id===doc.id)) {
        await bridge.dbStudioClose(opened.session_id); throw new Error('数据源或页签已变化，新会话已请求关闭');
      }
      doc.sessionId=opened.session_id; doc.transaction=opened.transaction||'idle'; return opened.session_id as string;
    })();
    openingSessions.set(doc.id,promise);
    try{return await promise;}finally{openingSessions.delete(doc.id);}
  }
  async function releaseSession(doc:QueryDocument=document.value):Promise<boolean> {
    if(doc.closing) return false;
    doc.closing=true;
    try {
      const opening=openingSessions.get(doc.id);
      if(opening) { try { await opening; } catch { /* Opening observes closing and releases its connection. */ } }
      const id=doc.sessionId; if(!id) {doc.transaction='idle';return true;}
      for(let attempt=0;attempt<180;attempt++) {
        const reply=await bridge.dbStudioClose(id);
        if(reply.closed) {if(doc.sessionId===id)doc.sessionId=null;doc.transaction='idle';return true;}
        await new Promise(resolve=>setTimeout(resolve,250));
      }
      doc.error='已请求关闭，但原生连接尚未确认停止；不能视为回滚完成。';return false;
    } catch(e) {doc.error=describeError(e);return false;}
    finally {doc.closing=false;}
  }
  async function sessionAction(action:string, options:Record<string,unknown>={}) {
    const doc=document.value;
    if(!lifecycleActive()||doc.busy||doc.closing||doc.pending||!doc.connId)return;
    doc.busy=true;doc.error='';
    try {
      const id=await ensureSession(doc);
      const reply=await bridge.dbStudioCall(id,{...options,action});
      if(reply.transaction)doc.transaction=reply.transaction;
    } catch(e) {doc.error=describeError(e);}
    finally {doc.busy=false;}
  }
  async function requestRunText(text:string,batch=false) {
    const doc=document.value;
    if(!lifecycleActive()||doc.busy||doc.pending||doc.closing)return;
    const gate=canRunQuery({backendReady:backendReady.value,connected:!!doc.connId,sql:text});
    if(!gate.ok){doc.error=gate.reason;return;}
    try {
      const statements=sqlStatements(text);
      if(!statements.length)throw new Error('没有可执行语句');
      if(!batch&&statements.length!==1)throw new Error('请选择单条语句，或使用运行脚本');
      if(requiresConfirm.value) {doc.pending=text;doc.pendingBatch=batch;return;}
      await execute(text,false,batch);
    }catch(e){doc.error=describeError(e);}
  }
  function selectBatchResult(index:number) {
    const result=document.value.batchResults[index];
    if(result){document.value.raw=result;document.value.result=buildResultView(result);}
  }
'''
    s = once(s, '  return {\n    documents,', helpers + '\n  return {\n    configs, activateDocument, requestRunText, sessionAction, releaseSession, selectBatchResult,\n    documents,')
    # Connecting a different form must not retarget an existing nonempty SQL document.
    s = once(s, '      doc.connId = res.conn_id;', '''      if (doc.connId !== res.conn_id && (doc.sql.trim() || doc.raw || doc.sessionId)) {
        if (documents.value.length >= 12) throw new Error('连接已保存；请关闭一个页签后再选择该数据源');
        const next = makeDoc(++sequence,res.conn_id);documents.value.push(next);activeDocument.value=next.id;
      } else doc.connId = res.conn_id;''')
    s = once(s, '    activeId.value=null;', '''    const old=document.value;
    if(old.sessionId){void releaseSession(old);return;}
    activeId.value=null;''')
    s = once(s, '      await bridge.dbDisconnect(id);', '''      const bound=documents.value.filter(d=>d.connId===id);
      const released=await Promise.all(bound.map(doc=>releaseSession(doc)));
      if(released.some(ok=>!ok)) throw new Error('仍有数据库会话未释放，不能删除连接');
      await bridge.dbDisconnect(id);''')
    write(path,s)

panel='src/capabilities/database/ui/DatabasePanel.vue'
s=read(panel)
if '<DatabaseWorkspace' not in s:
    s=once(s,'<script setup lang="ts">','<script setup lang="ts">\nimport DatabaseWorkspace from "./DatabaseWorkspace.vue";')
    start=s.index('    <section class="query">')
    end=s.index('\n    </section>\n    </div>\n  </div>\n</template>',start)
    s=s[:start]+'    <DatabaseWorkspace />'+s[end+len('\n    </section>'):]
    write(panel,s)
print('DEVELOPER_STUDIO_SOURCE_BOUND_DATABASE=APPLIED')
