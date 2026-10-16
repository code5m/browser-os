import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";
import { sqlStatements } from "../model/studioSql.mjs";
import { makeTableDocument, dirtyTable } from "../model/studioTable.mjs";
import { bridge } from "../../../bridge";
import type { DbConnectionConfig, DbQueryResult } from "../../../types";
import { redactSecrets } from '../../../utils/redact';
import { databaseLifecycleSnapshot, registerDatabaseCleanup } from "../lifecycle";
import {
  buildConnectPayload,
  buildObjectDetailSql,
  type DbObjectDetailMode,
  buildTablePreviewSql,
  buildResultView,
  canRunQuery,
  connectionLabel,
  decodeDbValue,
  emptyConnectionForm,
  validateConnectionForm,
  type DbConnectionForm,
  type DbConnectionSummary,
  type DbKind,
  type DbResultView,
  type DbRiskLevel,
  type ProductionVerdict,
} from "../../../utils/dbUi";

// M4-4 live wiring：A4 已在 bridge.rs / main.rs / ACL 落地 db_connect / db_query / db_disconnect，
// 且 A4 检查点明确「前端 wiring 归 A5」，故此处直接调用 src/bridge.ts 的类型化封装，
// 不再做「可选成员探测」。命令缺失时 Tauri invoke 会抛错，store 已按 fail-closed 兜底。

function describeError(e: unknown): string {
  const text = e instanceof Error ? e.message : typeof e === "string" ? e : JSON.stringify(e);
  return text ? redactSecrets(text).slice(0, 300) : "未知错误";
}

export const useDatabaseStore = defineStore("database", () => {
  const form = ref<DbConnectionForm>(emptyConnectionForm());
  const connections = ref<DbConnectionSummary[]>([]);
  type QueryDocument = { id:string; name:string; connId:string|null; sql:string; result:DbResultView|null; raw:DbQueryResult|null; busy:boolean; error:string; queryId:string|null; pending:string|null; tablePreview:{name:string;offset:number}|null; sessionId:string|null; transaction:string; batchResults:DbQueryResult[]; pendingBatch:boolean; closing:boolean; closeFailed:boolean; risk:DbRiskLevel; verdict:ProductionVerdict; tableData:ReturnType<typeof makeTableDocument>|null };
  const makeDoc = (n:number, connId:string|null):QueryDocument => ({ id:crypto.randomUUID(), name:`SQL ${n}`, connId, sql:'', result:null, raw:null, busy:false, error:'', queryId:null, pending:null, tablePreview:null, sessionId:null, transaction:'idle', batchResults:[], pendingBatch:false, closing:false, closeFailed:false, risk:'unknown', verdict:'Unknown', tableData:null });
  let sequence = 1;
  const documents = ref<QueryDocument[]>([makeDoc(sequence,null)]);
  const activeDocument = ref(documents.value[0].id);
  const document = computed(() => documents.value.find(d => d.id === activeDocument.value)!);
  const configs = ref<DbConnectionConfig[]>([]);
  const connecting = ref(false);
  const activeId = computed({ get:() => document.value.connId, set:(v:string|null) => { document.value.connId = v; } });
  const sql = computed({ get:() => document.value.sql, set:(v:string) => { document.value.sql = v; } });
  const result = computed({ get:() => document.value.result, set:(v:DbResultView|null) => { document.value.result = v; } });
  const busy = computed({ get:() => connecting.value || document.value.busy, set:(v:boolean) => { connecting.value = v; } });
  const error = computed({ get:() => document.value.error, set:(v:string) => { document.value.error = v; } });
  const schema = ref<{ name:string; type:string }[]>([]);
  const schemaBusy = ref(false);
  let schemaGeneration = 0;
  let columnsGeneration = 0;
  const selectedObject = ref("");
  const objectDetailMode = ref<DbObjectDetailMode>("columns");
  const objectDetailRows = ref<string[][]>([]);
  const columnsBusy = ref(false);
  // This is a validation marker, NOT an assertion of an idle live MySQL socket.
  const verifiedConnectionId = ref<string | null>(null);
  const recentlyVerified = computed(() => !!activeId.value && verifiedConnectionId.value === activeId.value);
  // SQL history is opt-in, bounded and session-only. Never persist queries to disk.
  const rememberQueries = ref(false);
  const queryHistory = ref<{id:string;connId:string;connection:string;sql:string;at:string;elapsed:string}[]>([]);
  const closePending = ref<string|null>(null);
  function lifecycleActive(): boolean {
    return databaseLifecycleSnapshot().active;
  }

  registerDatabaseCleanup(async () => {
    // A rejected cleanup blocks capability suspension. Never convert a failed
    // close acknowledgement into successful resource release.
    const owned = [...documents.value];
    const queryIds = owned.map(doc => doc.queryId).filter((id): id is string => !!id);
    const cancellations = await Promise.allSettled(queryIds.map(id => bridge.dbCancel(id)));
    const closures = await Promise.allSettled(owned.map(doc => releaseSession(doc)));
    if (closures.some(result => result.status === 'rejected' || !result.value) ||
        cancellations.some(result => result.status === 'rejected')) {
      throw new Error('DB_CLEANUP_NOT_CONFIRMED');
    }
    schemaGeneration += 1;
    columnsGeneration += 1;
    objectDetailRows.value = [];
    selectedObject.value = '';
    queryHistory.value = [];
    verifiedConnectionId.value = null;
    connecting.value = false;
    schemaBusy.value = false;
    for (const doc of owned) {
      doc.busy = false;
      doc.queryId = null;
      doc.pending = null;
    }
  });
  /** 风险等级由后端分类器给出（F4），前端只展示；未返回按 unknown 处理。 */
  const risk = computed({get:()=>document.value.risk,set:(value:DbRiskLevel)=>{document.value.risk=value;}});
  const verdict = computed({get:()=>document.value.verdict,set:(value:ProductionVerdict)=>{document.value.verdict=value;}});
  /** 待确认的 SQL：非空表示「已拦截、尚未发 IPC」 */
  const pendingSql = computed({ get:() => document.value.pending, set:(v:string|null) => { document.value.pending = v; } });

  const backendReady = computed(
    () =>
      typeof bridge.dbConnect === "function" &&
      typeof bridge.dbQuery === "function" &&
      typeof bridge.dbDisconnect === "function",
  );
  const connected = computed(() => !!activeId.value);
  const formIssues = computed(() => validateConnectionForm(form.value));
  const runGate = computed(() =>
    canRunQuery({ backendReady: backendReady.value, connected: connected.value, sql: sql.value }),
  );
  // 写权限开启的连接一律需二次确认（fail-closed 提示）；后端仍对每条语句重新分类与闸门。
  // 前端不自行分类（F4），故按连接能力而非语句风险触发确认，只读连接直接放行、由后端兜底。
  const requiresConfirm = computed(() => configs.value.find(c => c.id === activeId.value)?.allow_write ?? true);
  const confirmOpen = computed(() => pendingSql.value !== null);

  function resetForm() {
    form.value = emptyConnectionForm();
  }

  function setKind(kind: DbKind) {
    form.value.kind = kind;
  }

  function selectConnection(c: DbConnectionSummary) {
    if (document.value.busy || document.value.pending || document.value.closing) return;
    const cfg = configs.value.find(it => it.id === c.id && it.enabled);
    if (!cfg) return;
    const previous = document.value;
    // Never silently retarget existing SQL, results or a transaction.
    if (previous.connId !== c.id && (previous.sql.trim() || previous.raw || previous.sessionId || previous.tableData)) {
      const reusable = documents.value.find(d => d.connId === c.id && !d.busy && !d.pending);
      if (reusable) activeDocument.value = reusable.id;
      else {
        if (documents.value.length >= 12) { previous.error = '最多打开 12 个查询页签'; return; }
        const next = makeDoc(++sequence, c.id); documents.value.push(next); activeDocument.value = next.id;
      }
    } else document.value.connId = c.id;
    synchronizeDocumentSource();
  }

  async function refreshConnections() {
    if (!lifecycleActive()) return;
    const lifecycle = databaseLifecycleSnapshot();
    try {
      const next = await bridge.dbListConnections();
      const current = databaseLifecycleSnapshot();
      if (!current.active || current.generation !== lifecycle.generation) return;
      configs.value = next;
      connections.value = next.map(cfg => ({ id:cfg.id, name:cfg.name, kind:cfg.kind, label:connectionLabel(cfg), allowWrite:cfg.allow_write, enabled:cfg.enabled }));
    } catch(e) {
      const current = databaseLifecycleSnapshot();
      if (current.active && current.generation === lifecycle.generation) error.value = describeError(e);
    }
  }

  function newDocument() {
    if (documents.value.length >= 12) { error.value = '最多打开 12 个查询页签'; return; }
    const next = makeDoc(++sequence, activeId.value); documents.value.push(next); activeDocument.value = next.id;
  }
  async function closeDocument(id:string, confirmed=false) {
    const doc = documents.value.find(d => d.id === id);
    if (!doc || doc.busy || doc.closing) return;
    if ((doc.sql.trim() || doc.transaction !== 'idle' || dirtyTable(doc.tableData)) && !confirmed) { closePending.value = id; return; }
    closePending.value = null;
    if (!await releaseSession(doc)) return;
    documents.value = documents.value.filter(d => d.id !== id);
    if (!documents.value.length) documents.value.push(makeDoc(++sequence,null));
    if (activeDocument.value === id) activeDocument.value = documents.value[0].id;
  }

  async function cancelQuery() {
    const doc = document.value;
    try {
      if (doc.sessionId) {
        const response = await bridge.dbStudioCancel(doc.sessionId);
        if (response.cancel_requested) doc.error = '已请求取消；等待数据库停止后才会释放事务。';
        else if (!response.completed) doc.error = '查询正在启动，正在关闭该会话以确保停止。';
        if (!response.cancel_requested && !response.completed) void releaseSession(doc);
      } else if (doc.queryId && !await bridge.dbCancel(doc.queryId)) doc.error = '查询已完成或尚未开始';
    } catch(e) { doc.error = describeError(e); }
  }

  async function refreshSchema() {
    if (!lifecycleActive()) return;
    const lifecycle = databaseLifecycleSnapshot();
    const id = activeId.value; schema.value = [];
    const generation = ++schemaGeneration;
    const kind = configs.value.find(c => c.id === id)?.kind;
    if (!id || (kind !== 'sqlite' && kind !== 'mysql')) return;
    const schemaSql = kind === 'mysql'
      ? "SELECT TABLE_NAME,TABLE_TYPE FROM information_schema.tables WHERE TABLE_SCHEMA=DATABASE() ORDER BY TABLE_NAME LIMIT 1000"
      : "SELECT name, type FROM sqlite_schema WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' ORDER BY name";
    schemaBusy.value = true;
    try {
      const data = await bridge.dbQuery({ conn_id:id, sql:schemaSql, confirm_write:false });
      const current = databaseLifecycleSnapshot();
      if (current.active && current.generation === lifecycle.generation && activeId.value === id && generation === schemaGeneration) {
        schema.value = data.rows.map(row => ({name:String(decodeDbValue(row[0])),type:String(decodeDbValue(row[1]))}));
      }
    } catch(e) {
      const current = databaseLifecycleSnapshot();
      if (current.active && current.generation === lifecycle.generation && activeId.value === id && generation === schemaGeneration) error.value = describeError(e);
    } finally {
      const current = databaseLifecycleSnapshot();
      if (current.active && current.generation === lifecycle.generation && generation === schemaGeneration) schemaBusy.value = false;
    }
  }
  async function selectObject(name: string, mode: DbObjectDetailMode = "columns") {
    if (!lifecycleActive()) return;
    const id = activeId.value;
    const kind = configs.value.find(c => c.id === id)?.kind;
    const isView = schema.value.some(o => o.name === name && o.type.toLowerCase().includes("view"));
    selectedObject.value = name;
    objectDetailMode.value = mode;
    objectDetailRows.value = [];
    const generation = ++columnsGeneration;
    if (!id || (kind !== "mysql" && kind !== "sqlite")) return;
    columnsBusy.value = true;
    try {
      const data = await bridge.dbQuery({
        conn_id: id, sql: buildObjectDetailSql(name, kind, mode, isView), confirm_write: false,
      });
      if (generation === columnsGeneration && activeId.value === id && selectedObject.value === name && mode === objectDetailMode.value) {
        objectDetailRows.value = data.rows.map(row => row.map(cell => String(decodeDbValue(cell) ?? "NULL")));
      }
    } catch (e) {
      if (generation === columnsGeneration) error.value = describeError(e);
    } finally {
      if (generation === columnsGeneration) columnsBusy.value = false;
    }
  }

  function openTablePage(offset: number) {
    const doc = document.value;
    const table = doc.tablePreview;
    const kind = configs.value.find(c => c.id === doc.connId)?.kind;
    if (!table || !doc.connId || doc.busy || doc.pending || (kind !== "sqlite" && kind !== "mysql")) return;
    try {
      doc.sql = buildTablePreviewSql(table.name, kind, offset);
      table.offset = offset;
      requestRun();
    } catch (e) { doc.error = describeError(e); }
  }
  function nextTablePage() {
    const table = document.value.tablePreview;
    if (!table || !result.value || result.value.rowCount < 100) return;
    openTablePage(table.offset + 100);
  }
  function previousTablePage() {
    const table = document.value.tablePreview;
    if (!table || table.offset < 100) return;
    openTablePage(table.offset - 100);
  }
  function explainCurrentSql() {
    const text = sql.value.trim().replace(/;\s*$/, "");
    const kind = configs.value.find(c => c.id === activeId.value)?.kind;
    // EXPLAIN ANALYZE executes statements; never construct that variant.
    if (!/^select\b/i.test(text) || kind !== "mysql" || document.value.busy) {
      error.value = "仅支持对当前 MySQL SELECT 执行 EXPLAIN（不使用 ANALYZE）";
      return;
    }
    if (documents.value.length >= 12) { error.value = "最多打开 12 个查询页签"; return; }
    newDocument();
    document.value.name = "EXPLAIN 计划";
    sql.value = "EXPLAIN " + text;
    requestRun();
  }
  /** Double-click a table: open a real bounded read-only preview and execute it.
   * The ordinary requestRun gate still applies, including confirmations for
   * connections that allow writes.
   */
  function previewTable(name: string) { openDataEditor(name); }

  function reuseHistory(item: {sql:string;connection:string;connId?:string}) {
    if (!item.connId || !configs.value.some(c=>c.id===item.connId && c.enabled)) {
      error.value = '历史数据源已删除或不可用，不能将 SQL 静默切换到当前库。'; return;
    }
    if (documents.value.length >= 12) { error.value = '最多打开 12 个查询页签'; return; }
    const next=makeDoc(++sequence,item.connId); next.name='历史 SQL'; next.sql=item.sql;
    documents.value.push(next); activeDocument.value=next.id;
  }

  function clearQueryHistory() { queryHistory.value = []; }
  function setRememberQueries(value: boolean) {
    rememberQueries.value = value;
    if (!value) clearQueryHistory(); // never retain previous session SQL after opting out
  }

  /**
   * 建立连接。`password` 只作为**瞬时参数**传入后端（后端落 Keyring，键 `db:<conn_id>`），
   * 绝不写入 form / store / localStorage（F2）。
   */
  async function connect(password: string) {
    if (!lifecycleActive()) return false;
    const lifecycle = databaseLifecycleSnapshot();
    const doc = document.value;
    if (connecting.value || doc.busy || doc.pending) return false;
    error.value = "";
    if (formIssues.value.length) {
      error.value = `表单校验未通过：${formIssues.value[0].message}`;
      return false;
    }
    if (!backendReady.value) {
      error.value = "后端数据库命令未就绪";
      return false;
    }
    busy.value = true;
    try {
      // buildConnectPayload 已含 id/created_at/updated_at（与 DbConnectionConfig 对齐），
      // 且结构性不含 password（F2）。密码只作为瞬时参数传入封装，不进 payload/store/表单。
      const cfg = buildConnectPayload(form.value) as unknown as DbConnectionConfig;
      const res = await bridge.dbConnect(cfg, password);
      const current = databaseLifecycleSnapshot();
      if (!current.active || current.generation !== lifecycle.generation) return false;
      if (doc.connId !== res.conn_id && (doc.sql.trim() || doc.raw || doc.sessionId)) {
        if (documents.value.length >= 12) throw new Error('连接已保存；请关闭一个页签后再选择该数据源');
        const next = makeDoc(++sequence,res.conn_id);documents.value.push(next);activeDocument.value=next.id;
      } else doc.connId = res.conn_id;
      verifiedConnectionId.value = res.conn_id;
      risk.value = "unknown";
      verdict.value = "Unknown";
      result.value = null;
      await refreshConnections();
      await refreshSchema();
      return !!activeId.value;
    } catch (e) {
      const current = databaseLifecycleSnapshot();
      if (current.active && current.generation === lifecycle.generation) error.value = describeError(e);
      return false;
    } finally {
      const current = databaseLifecycleSnapshot();
      if (current.active && current.generation === lifecycle.generation) busy.value = false;
    }
  }

  function deselectConnection() {
    // DbPool is transient. This only closes the selected UI session;
    // saved non-secret configuration and the Keyring credential are retained.
    if (document.value.busy || document.value.pending) return;
    const old=document.value;
    if(old.sessionId){void releaseSession(old);return;}
    activeId.value=null;
    verifiedConnectionId.value=null;
    schemaGeneration++;
    columnsGeneration++;
    schema.value=[];
    objectDetailRows.value=[];
    selectedObject.value="";
    document.value.raw=null;
    result.value=null;
    resetForm();
  }

  async function disconnect() {
    if (!lifecycleActive() || !activeId.value) return;
    const lifecycle = databaseLifecycleSnapshot();
    error.value = "";
    const id = activeId.value;
    if (verifiedConnectionId.value === id) verifiedConnectionId.value = null;
    risk.value = "unknown";
    verdict.value = "Unknown";
    pendingSql.value = null;
    if (!backendReady.value) return;
    try {
      const bound=documents.value.filter(d=>d.connId===id);
      const released=await Promise.all(bound.map(doc=>releaseSession(doc)));
      if(released.some(ok=>!ok)) throw new Error('仍有数据库会话未释放，不能删除连接');
      await bridge.dbDisconnect(id);
      for (const doc of documents.value) if (doc.connId === id) { doc.connId = null; doc.pending = null; }
      schema.value = [];
      resetForm();
    } catch (e) {
      const current = databaseLifecycleSnapshot();
      if (current.active && current.generation === lifecycle.generation) error.value = describeError(e);
    } finally {
      const current = databaseLifecycleSnapshot();
      if (current.active && current.generation === lifecycle.generation) void refreshConnections();
    }
  }

  function requestRun() { void requestRunText(sql.value, false); }

  function cancelConfirm() { document.value.pending = null; document.value.pendingBatch = false; }

  function confirmRun() {
    if (!lifecycleActive()) return;
    const doc=document.value, text=doc.pending, batch=doc.pendingBatch;
    doc.pending=null; doc.pendingBatch=false;
    if(text) void execute(text,true,batch);
  }

  async function execute(text:string, confirmed:boolean, batch=false) {
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
      const structured=sessionFailure(doc,e);
      if(!structured && doc.sessionId && !doc.closing) {
        try { const state=await bridge.dbStudioCall(doc.sessionId,{action:'status'}); doc.transaction=state.transaction; }
        catch { doc.transaction='unknown'; doc.error+='；会话状态未知，请关闭会话，不要重复执行写入。'; }
      }
    } finally { doc.busy=false; doc.queryId=null; }
  }

  function clearResult() {
    result.value = null;
    document.value.raw = null;
    error.value = "";
    pendingSql.value = null;
  }


  // DEVELOPER_STUDIO_SOURCE_BOUND_DOCUMENTS
  const openingSessions = new Map<string, Promise<string>>();
  const closingSessions = new Map<string, Promise<boolean>>();
  function sessionFailure(doc:QueryDocument, error:unknown):boolean {
    // Preserve the string-error IPC ABI while retaining native failure facts.
    try {
      const fault = typeof error === 'string' ? JSON.parse(error) : error;
      if (fault && typeof fault === 'object' && fault.kind === 'database_session_fault') {
        doc.error = String(fault.code) + ': ' + String(fault.message);
        if (['idle','read_only','read_write','failed','unknown'].includes(fault.transaction)) doc.transaction = fault.transaction;
        if (fault.discard_connection === true) doc.closeFailed = true;
        return true;
      }
    } catch { /* Legacy errors are handled below, never guessed successful. */ }
    doc.error = describeError(error);
    return false;
  }
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
    if(doc.closeFailed || doc.transaction === 'unknown') throw new Error('DB_SESSION_CLOSE_OR_OUTCOME_UNCONFIRMED');
    if(doc.sessionId) return doc.sessionId;
    const existing=openingSessions.get(doc.id); if(existing) return existing;
    const connId=doc.connId,lifecycle=databaseLifecycleSnapshot();
    if(!connId) throw new Error('请先选择数据源');
    const promise=(async()=>{
      const opened=await bridge.dbStudioOpen(connId);
      const current=databaseLifecycleSnapshot();
      doc.sessionId=opened.session_id;
      if(doc.closing||!current.active||current.generation!==lifecycle.generation||doc.connId!==connId||!documents.value.some(d=>d.id===doc.id)) {
        await closeNativeSession(doc, opened.session_id); throw new Error('数据源或页签已变化，新会话已请求关闭');
      }
      doc.sessionId=opened.session_id; doc.transaction=opened.transaction||'idle'; return opened.session_id as string;
    })();
    openingSessions.set(doc.id,promise);
    try{return await promise;}finally{openingSessions.delete(doc.id);}
  }
  async function closeNativeSession(doc:QueryDocument, id:string):Promise<boolean> {
    try {
      for(let attempt=0;attempt<180;attempt++) {
        const reply=await bridge.dbStudioClose(id);
        if(reply.closed === true) {
          if(doc.sessionId === id) doc.sessionId=null;
          // Closing a socket cannot prove an ambiguous COMMIT was rolled back.
          if(doc.transaction !== 'unknown') doc.transaction='idle';
          doc.closeFailed=false;
          return true;
        }
        await new Promise(resolve=>setTimeout(resolve,250));
      }
      doc.error='DB_SESSION_CLOSE_NOT_CONFIRMED';
    } catch(error) { sessionFailure(doc,error); }
    doc.closeFailed=true;
    doc.transaction='unknown';
    return false;
  }
  function releaseSession(doc:QueryDocument=document.value):Promise<boolean> {
    const running=closingSessions.get(doc.id);
    if(running) return running;
    doc.closing=true;
    const closing=(async()=>{
      try {
        const opening=openingSessions.get(doc.id);
        if(opening) { try { await opening; } catch { /* Failed opening retains an unclosed session ID. */ } }
        const id=doc.sessionId;
        if(!id) return !doc.closeFailed;
        return await closeNativeSession(doc,id);
      } finally { doc.closing=false; }
    })();
    closingSessions.set(doc.id,closing);
    void closing.finally(()=>closingSessions.delete(doc.id));
    return closing;
  }
  async function sessionAction(action:string, options:Record<string,unknown>={}) {
    const doc=document.value;
    if(!lifecycleActive()||doc.busy||doc.closing||doc.pending||!doc.connId)return;
    doc.busy=true;doc.error='';
    try {
      const id=await ensureSession(doc);
      const reply=await bridge.dbStudioCall(id,{...options,action});
      if(reply.transaction)doc.transaction=reply.transaction;
    } catch(e) {sessionFailure(doc,e);}
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

  // DEVELOPER_STUDIO_TABLE_DOCUMENTS
  function openDataEditor(name:string) {
    const connId=activeId.value;
    if(!connId || !schema.value.some(item=>item.name===name)) { error.value='请从当前数据源对象树选择表或视图'; return; }
    const existing=documents.value.find(doc=>doc.connId===connId && doc.tableData?.name===name && !doc.closing);
    if(existing) { activeDocument.value=existing.id; return; }
    if(documents.value.length>=12) { error.value='最多打开 12 个查询页签'; return; }
    const next=makeDoc(++sequence,connId); next.name=name+' · 数据'; next.tableData=makeTableDocument(name);
    documents.value.push(next);activeDocument.value=next.id;
  }
  async function tableOperation(action:string, table:Record<string,unknown>, confirmed=false, documentId=activeDocument.value):Promise<any|null> {
    const doc=documents.value.find(item=>item.id===documentId),lifecycle=databaseLifecycleSnapshot();
    if(!lifecycle.active || !doc || !doc.connId || doc.busy || doc.closing || doc.pending || !['table_read','table_preview','table_apply','table_cancel'].includes(action))return null;
    const connId=doc.connId;doc.busy=true;doc.error='';
    try {
      const id=await ensureSession(doc);
      const reply=await bridge.dbStudioCall(id,{action,table,confirmed});
      const current=databaseLifecycleSnapshot();
      if(!current.active || current.generation!==lifecycle.generation || doc.connId!==connId || !documents.value.some(item=>item.id===documentId)) return null;
      if(reply.transaction)doc.transaction=reply.transaction;
      return reply;
    } catch(e) {
      const structured=sessionFailure(doc,e);
      if(!structured && doc.sessionId && !doc.closing) {
        try { const state=await bridge.dbStudioCall(doc.sessionId,{action:'status'});doc.transaction=state.transaction; }
        catch { doc.transaction='unknown';doc.error+='；会话状态未知，请关闭会话，不要重试写操作。'; }
      }
      return null;
    } finally {doc.busy=false;}
  }

  return {
    openDataEditor, tableOperation,
    configs, activateDocument, requestRunText, sessionAction, releaseSession, selectBatchResult,
    documents, activeDocument, document, closePending, newDocument, closeDocument, cancelQuery,
    schema, schemaBusy, selectedObject, columnsBusy, objectDetailRows,
    objectDetailMode, selectObject, refreshSchema, previewTable, nextTablePage, previousTablePage,
    explainCurrentSql,
    recentlyVerified, rememberQueries, setRememberQueries, queryHistory, reuseHistory, clearQueryHistory,
    form,
    connections,
    activeId,
    sql,
    result,
    busy,
    error,
    risk,
    verdict,
    pendingSql,
    backendReady,
    connected,
    formIssues,
    runGate,
    requiresConfirm,
    confirmOpen,
    resetForm,
    setKind,
    selectConnection,
    refreshConnections,
    connect,
    deselectConnection,
    disconnect,
    requestRun,
    confirmRun,
    cancelConfirm,
    clearResult,
  };
});
