import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { bridge } from "../../../bridge";
import type { DbConnectionConfig, DbQueryResult } from "../../../types";
import { redactSecrets } from '../../../utils/redact';
import { databaseLifecycleSnapshot, registerDatabaseCleanup } from "../lifecycle";
import {
  buildConnectPayload,
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
  type QueryDocument = { id:string; name:string; connId:string|null; sql:string; result:DbResultView|null; raw:DbQueryResult|null; busy:boolean; error:string; queryId:string|null; pending:string|null };
  const makeDoc = (n:number, connId:string|null):QueryDocument => ({ id:crypto.randomUUID(), name:`SQL ${n}`, connId, sql:'', result:null, raw:null, busy:false, error:'', queryId:null, pending:null });
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
  const closePending = ref<string|null>(null);
  function lifecycleActive(): boolean {
    return databaseLifecycleSnapshot().active;
  }

  registerDatabaseCleanup(async () => {
    schemaGeneration += 1;
    pendingSql.value = null;
    connecting.value = false;
    schemaBusy.value = false;
    const queryIds = documents.value.map((doc) => doc.queryId).filter((id): id is string => !!id);
    await Promise.allSettled(queryIds.map((id) => bridge.dbCancel(id)));
    for (const doc of documents.value) {
      doc.busy = false;
      doc.queryId = null;
      doc.pending = null;
    }
  });
  /** 风险等级由后端分类器给出（F4），前端只展示；未返回按 unknown 处理。 */
  const risk = ref<DbRiskLevel>("unknown");
  const verdict = ref<ProductionVerdict>("Unknown");
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
    if (document.value.busy || document.value.pending) return;
    const cfg = configs.value.find(it => it.id === c.id);
    if (!cfg) return;
    activeId.value = c.id;
    form.value = {
      ...emptyConnectionForm(),
      id: c.id,
      name: c.name,
      kind: c.kind,
      allowWrite: c.allowWrite,
      enabled: c.enabled,
      database: cfg.database, host:cfg.host || '', portText:cfg.port ? String(cfg.port) : '', username:cfg.username || '', sslMode:cfg.ssl_mode,
      productionHint:cfg.production_hint === true ? 'yes' : cfg.production_hint === false ? 'no' : '',
    };
    result.value = null; document.value.raw = null;
    void refreshSchema();
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
  function closeDocument(id:string, confirmed=false) {
    const doc = documents.value.find(d => d.id === id); if (!doc || doc.busy) return;
    if (doc.sql.trim() && !confirmed) { closePending.value = id; return; }
    closePending.value = null;
    documents.value = documents.value.filter(d => d.id !== id);
    if (!documents.value.length) documents.value.push(makeDoc(++sequence,null));
    if (activeDocument.value === id) activeDocument.value = documents.value[0].id;
  }
  async function cancelQuery() {
    if (!lifecycleActive()) return;
    const doc = document.value; if (!doc.queryId) return;
    try { if (!await bridge.dbCancel(doc.queryId)) doc.error = '查询已完成或尚未开始'; }
    catch(e) { doc.error = describeError(e); }
  }
  async function refreshSchema() {
    if (!lifecycleActive()) return;
    const lifecycle = databaseLifecycleSnapshot();
    const id = activeId.value; schema.value = [];
    const generation = ++schemaGeneration;
    if (!id || configs.value.find(c => c.id === id)?.kind !== 'sqlite') return;
    schemaBusy.value = true;
    try {
      const data = await bridge.dbQuery({ conn_id:id, sql:"SELECT name, type FROM sqlite_schema WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' ORDER BY name", confirm_write:false });
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
  function previewTable(name:string) {
    if (documents.value.length >= 12) { error.value = '最多打开 12 个查询页签'; return; }
    newDocument();
    if (!document.value.busy) sql.value = `SELECT * FROM "${name.replace(/"/g,'""')}" LIMIT 100`;
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
      doc.connId = res.conn_id;
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

  async function disconnect() {
    if (!lifecycleActive() || !activeId.value) return;
    const lifecycle = databaseLifecycleSnapshot();
    error.value = "";
    const id = activeId.value;
    risk.value = "unknown";
    verdict.value = "Unknown";
    pendingSql.value = null;
    if (!backendReady.value) return;
    try {
      await bridge.dbDisconnect(id);
      for (const doc of documents.value) if (doc.connId === id) { doc.connId = null; doc.pending = null; }
      schema.value = [];
    } catch (e) {
      const current = databaseLifecycleSnapshot();
      if (current.active && current.generation === lifecycle.generation) error.value = describeError(e);
    } finally {
      const current = databaseLifecycleSnapshot();
      if (current.active && current.generation === lifecycle.generation) void refreshConnections();
    }
  }

  function requestRun() {
    if (!lifecycleActive() || busy.value || confirmOpen.value) return;
    const gate = runGate.value;
    if (!gate.ok) {
      error.value = gate.reason;
      return;
    }
    // 确认前不发 IPC：仅挂起待确认 SQL
    if (requiresConfirm.value) {
      pendingSql.value = sql.value;
      return;
    }
    void execute(sql.value, false);
  }

  function cancelConfirm() {
    pendingSql.value = null;
  }

  function confirmRun() {
    if (!lifecycleActive()) return;
    const text = pendingSql.value;
    pendingSql.value = null;
    if (text) void execute(text, true);
  }

  async function execute(text: string, confirmed: boolean) {
    if (!lifecycleActive()) return;
    const lifecycle = databaseLifecycleSnapshot();
    const doc = document.value;
    if (!backendReady.value || !doc.connId || doc.busy) return;
    doc.busy = true; doc.error = ''; doc.queryId = crypto.randomUUID();
    try {
      // 仅当本次执行来自二次确认对话框时视为用户显式确认写操作（fail-closed）；
      // timeout_secs 传 null 表示沿用后端默认（domain.rs DB_DEFAULT_QUERY_TIMEOUT_SECS）。
      const res = await bridge.dbQuery({
        conn_id: doc.connId,
        sql: text,
        timeout_secs: null,
        confirm_write: confirmed,
        query_id: doc.queryId,
      });
      // 风险等级与生产判定以后端回带为准；未回带时 buildResultView 已按 unknown / Unknown 兜底（fail-closed）
      const current = databaseLifecycleSnapshot();
      if (!current.active || current.generation !== lifecycle.generation) return;
      const view = buildResultView(res);
      doc.result = view; doc.raw = res;
      risk.value = view.risk;
      verdict.value = view.verdict;
    } catch (e) {
      const current = databaseLifecycleSnapshot();
      if (current.active && current.generation === lifecycle.generation) doc.error = describeError(e);
    } finally {
      const current = databaseLifecycleSnapshot();
      if (current.active && current.generation === lifecycle.generation) {
        doc.busy = false; doc.queryId = null;
      }
    }
  }

  function clearResult() {
    result.value = null;
    document.value.raw = null;
    error.value = "";
    pendingSql.value = null;
  }

  return {
    documents, activeDocument, document, closePending, newDocument, closeDocument, cancelQuery, schema, schemaBusy, refreshSchema, previewTable,
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
    disconnect,
    requestRun,
    confirmRun,
    cancelConfirm,
    clearResult,
  };
});
