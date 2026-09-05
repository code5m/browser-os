import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { bridge } from "../bridge";
import type { DbConnectionConfig } from "../types";
import {
  buildConnectPayload,
  buildResultView,
  canRunQuery,
  emptyConnectionForm,
  validateConnectionForm,
  type DbConnectionForm,
  type DbConnectionSummary,
  type DbKind,
  type DbResultView,
  type DbRiskLevel,
  type ProductionVerdict,
} from "../utils/dbUi";

// M4-4 live wiring：A4 已在 bridge.rs / main.rs / ACL 落地 db_connect / db_query / db_disconnect，
// 且 A4 检查点明确「前端 wiring 归 A5」，故此处直接调用 src/bridge.ts 的类型化封装，
// 不再做「可选成员探测」。命令缺失时 Tauri invoke 会抛错，store 已按 fail-closed 兜底。

function describeError(e: unknown): string {
  const text = e instanceof Error ? e.message : typeof e === "string" ? e : JSON.stringify(e);
  return text ? text.slice(0, 300) : "未知错误";
}

export const useDatabaseStore = defineStore("database", () => {
  const form = ref<DbConnectionForm>(emptyConnectionForm());
  const connections = ref<DbConnectionSummary[]>([]);
  const activeId = ref<string | null>(null);
  const sql = ref("");
  const result = ref<DbResultView | null>(null);
  const busy = ref(false);
  const error = ref("");
  /** 风险等级由后端分类器给出（F4），前端只展示；未返回按 unknown 处理。 */
  const risk = ref<DbRiskLevel>("unknown");
  const verdict = ref<ProductionVerdict>("Unknown");
  /** 待确认的 SQL：非空表示「已拦截、尚未发 IPC」 */
  const pendingSql = ref<string | null>(null);

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
  const requiresConfirm = computed(() => form.value.allowWrite);
  const confirmOpen = computed(() => pendingSql.value !== null);

  function resetForm() {
    form.value = emptyConnectionForm();
  }

  function setKind(kind: DbKind) {
    form.value.kind = kind;
  }

  function selectConnection(c: DbConnectionSummary) {
    form.value = {
      ...emptyConnectionForm(),
      id: c.id,
      name: c.name,
      kind: c.kind,
      allowWrite: c.allowWrite,
      enabled: c.enabled,
    };
  }

  function refreshConnections() {
    // A4 未提供连接列表命令（M4-4 聚焦「即连即查」），当前无 live 列表可拉取。
    // 保留空实现以便将来接入 db_list_connections 时无需改动调用点。
  }

  /**
   * 建立连接。`password` 只作为**瞬时参数**传入后端（后端落 Keyring，键 `db:<conn_id>`），
   * 绝不写入 form / store / localStorage（F2）。
   */
  async function connect(password: string) {
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
      activeId.value = res.conn_id;
      risk.value = "unknown";
      verdict.value = "Unknown";
      result.value = null;
      refreshConnections();
      return !!activeId.value;
    } catch (e) {
      error.value = describeError(e);
      return false;
    } finally {
      busy.value = false;
    }
  }

  async function disconnect() {
    if (!activeId.value) return;
    error.value = "";
    const id = activeId.value;
    activeId.value = null;
    result.value = null;
    risk.value = "unknown";
    verdict.value = "Unknown";
    pendingSql.value = null;
    if (!backendReady.value) return;
    try {
      await bridge.dbDisconnect(id);
    } catch (e) {
      error.value = describeError(e);
    } finally {
      refreshConnections();
    }
  }

  function requestRun() {
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
    const text = pendingSql.value;
    pendingSql.value = null;
    if (text) void execute(text, true);
  }

  async function execute(text: string, confirmed: boolean) {
    if (!backendReady.value || !activeId.value) return;
    busy.value = true;
    error.value = "";
    try {
      // 仅当本次执行来自二次确认对话框时视为用户显式确认写操作（fail-closed）；
      // timeout_secs 传 null 表示沿用后端默认（domain.rs DB_DEFAULT_QUERY_TIMEOUT_SECS）。
      const res = await bridge.dbQuery({
        conn_id: activeId.value,
        sql: text,
        timeout_secs: null,
        confirm_write: confirmed,
      });
      // 风险等级与生产判定以后端回带为准；未回带时 buildResultView 已按 unknown / Unknown 兜底（fail-closed）
      const view = buildResultView(res);
      result.value = view;
      risk.value = view.risk;
      verdict.value = view.verdict;
    } catch (e) {
      error.value = describeError(e);
    } finally {
      busy.value = false;
    }
  }

  function clearResult() {
    result.value = null;
    error.value = "";
    pendingSql.value = null;
  }

  return {
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
