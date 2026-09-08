// M4-4 数据库面板 UI 纯逻辑层（无 Vue / bridge 运行时依赖）。
//
// 契约依据（均已冻结/已落地，禁止自行发明口径）：
//   - `src-tauri/src/domain.rs`：`SupportedDb` / `DbConnectionConfig`（**结构性无 password 字段**，F2）
//     / `DbSslMode` / `DbValue`（`#[serde(rename_all="snake_case")]` 带标签枚举）
//     / `DbQueryResult`（`query_id` `columns` `rows` `row_count` `truncated` `field_truncated`
//     `limit_hit` `elapsed_ms` `state`）/ `DbLimitKind` / `DbQueryState`。
//   - `src-tauri/src/security_policy.rs`：`SqlRiskClass`（Read/Write/Ddl/Admin/Unknown）、
//     `ProductionVerdict`（Production/NonProduction/Unknown，`Unknown` ⇒ 拒绝写）。
//   - `logs/checkpoints/M4-1.c-20260905-2255.md`：上限数值表与「禁止静默截断」规则。
//
// 命令层（A4 已落地）：`db_connect` / `db_query` / `db_disconnect` 已注册到 `bridge.rs` 与 ACL
// （命令名沿用 A1 冻结）；`src/stores/useDatabaseStore.ts` 直接调用 `bridge.ts` 的类型化封装，
// 并以「命令是否存在」兜底（后端未就绪时不发 IPC）。

export type DbKind = "sqlite" | "mysql" | "postgres";
export type DbSslMode = "disable" | "prefer" | "require";
export type ProductionVerdict = "Production" | "NonProduction" | "Unknown";
export type ProductionHint = "" | "yes" | "no";
/** 后端 `SqlRiskClass` 的镜像（含 Admin 档）；前端只做着色与文案，不自行分类（F4）。 */
export type DbRiskLevel = "read" | "write" | "ddl" | "admin" | "unknown";
/** 后端 `DbLimitKind`（snake_case 序列化） */
export type DbLimitKind = "rows" | "bytes" | "field";
/** 后端 `DbQueryState`（snake_case 序列化） */
export type DbQueryState = "completed" | "cancelled" | "timeout" | "failed";
export type DbScalar = null | boolean | number | string;

/** M4-1.c §1 上限数值表（冻结、前端不可调；此处仅用于「超限提示」文案，不作服务端判定）。 */
export const DB_LIMITS = {
  maxRows: 1000,
  maxResultBytes: 4 * 1024 * 1024,
  maxTextFieldBytes: 64 * 1024,
  maxSqlBytes: 64 * 1024,
  defaultTimeoutSecs: 30,
  maxTimeoutSecs: 600,
} as const;

export interface DbConnectionForm {
  id: string | null;
  name: string;
  kind: DbKind;
  /** SQLite 忽略 */
  host: string;
  /** 文本态（空 = 用 `kind` 默认端口），避免 number NaN 污染表单 */
  portText: string;
  /** SQLite = 文件路径；其余 = 库名 */
  database: string;
  /** SQLite 忽略 */
  username: string;
  sslMode: DbSslMode;
  /** 写权限显式开关，缺省 false = 写默认拒绝（F1） */
  allowWrite: boolean;
  productionHint: ProductionHint;
  enabled: boolean;
}

export interface DbIssue {
  field: string;
  message: string;
}

export interface DbConnectionSummary {
  id: string;
  name: string;
  kind: DbKind;
  /** 展示用；**永不**含凭据或完整 DSN */
  label: string;
  allowWrite: boolean;
  enabled: boolean;
}

/** `DbQueryResult` 的 TS 镜像（字段名与后端一致，snake_case） */
export interface DbResultSet {
  query_id: string;
  columns: string[];
  rows: unknown[][];
  row_count: number;
  truncated: boolean;
  field_truncated: boolean;
  limit_hit?: DbLimitKind | null;
  elapsed_ms: number;
  state: DbQueryState;
  /** 后端如随结果回带风险等级（非冻结项），前端按 unknown 兜底 */
  risk?: DbRiskLevel | null;
  production_verdict?: ProductionVerdict | null;
}

export interface DbResultView {
  columns: string[];
  /** 已解码并格式化为展示字符串的单元格 */
  rows: string[][];
  rowCount: number;
  truncated: boolean;
  fieldTruncated: boolean;
  limitHit: DbLimitKind | null;
  state: DbQueryState;
  elapsedLabel: string;
  rowLabel: string;
  stateLabel: string;
  risk: DbRiskLevel;
  verdict: ProductionVerdict;
  /** 非静默截断：任何不完整都必须在 UI 上显式呈现（M4-1.c §1 规则 2） */
  warnings: string[];
}

export function emptyConnectionForm(): DbConnectionForm {
  return {
    id: null,
    name: "",
    kind: "sqlite",
    host: "",
    portText: "",
    database: "",
    username: "",
    sslMode: "prefer",
    allowWrite: false,
    productionHint: "",
    enabled: true,
  };
}

export function kindLabel(kind: DbKind): string {
  if (kind === "sqlite") return "SQLite";
  if (kind === "mysql") return "MySQL";
  return "PostgreSQL";
}

export function defaultPortFor(kind: DbKind): number | null {
  if (kind === "mysql") return 3306;
  if (kind === "postgres") return 5432;
  return null;
}

export function kindNeedsCredentials(kind: DbKind): boolean {
  return kind !== "sqlite";
}

export function databaseFieldLabel(kind: DbKind): string {
  return kind === "sqlite" ? "文件路径" : "数据库名";
}

export interface DbFormFields {
  host: boolean;
  port: boolean;
  username: boolean;
  ssl: boolean;
}

export function visibleFields(kind: DbKind): DbFormFields {
  const net = kindNeedsCredentials(kind);
  return { host: net, port: net, username: net, ssl: net };
}

export function validateConnectionForm(form: DbConnectionForm): DbIssue[] {
  const issues: DbIssue[] = [];
  const name = form.name.trim();
  const net = kindNeedsCredentials(form.kind);

  if (!name) issues.push({ field: "name", message: "连接名不能为空" });
  else if (utf8Bytes(name) > 128) issues.push({ field: "name", message: "连接名过长（上限 128 字节）" });

  if (!form.database.trim()) {
    issues.push({ field: "database", message: `${databaseFieldLabel(form.kind)}不能为空` });
  }

  if (net) {
    if (!form.host.trim()) issues.push({ field: "host", message: "主机不能为空" });
    if (!form.username.trim()) issues.push({ field: "username", message: "用户名不能为空" });
    const port = form.portText.trim();
    if (port) {
      const n = Number(port);
      if (!Number.isInteger(n) || n < 1 || n > 65535) {
        issues.push({ field: "portText", message: "端口必须是 1~65535 的整数" });
      }
    }
  }

  return issues;
}

/**
 * 生成新的连接 ID（仅客户端标识，用于 Keyring 键 `db:<conn_id>`，非安全敏感值；
 * 安全性由后端对每条语句的重新分类/闸门决定，与前端的 id 不可预测性无关）。
 * 优先用 crypto.randomUUID，老环境退化为 v4 形态随机数（仅保证唯一性）。
 */
export function newConnectionId(): string {
  const g = globalThis as { crypto?: { randomUUID?: () => string } };
  if (g.crypto && typeof g.crypto.randomUUID === "function") return g.crypto.randomUUID();
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (ch) => {
    const r = (Math.random() * 16) | 0;
    const v = ch === "x" ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

/**
 * 构造连接配置 payload（字段名与 `domain.rs::DbConnectionConfig` 一致，snake_case）。
 * 含 A4 后端要求的结构字段 `id` / `created_at` / `updated_at`（由 form.id / now 解析，
 * 缺省自动生成，使重连可复用既有 id 以对齐 Keyring 键）。
 * **结构性不含 password / dsn / connection_string**（F2）——不是「写前清空」，而是根本不存在这些键；
 * 密码由调用方以独立参数瞬时传入后端（落 Keyring，键 `db:<conn_id>`），不进本 payload、不进表单、不进 store。
 */
export function buildConnectPayload(
  form: DbConnectionForm,
  opts: { id?: string; now?: Date } = {},
): Record<string, unknown> {
  const net = kindNeedsCredentials(form.kind);
  const port = form.portText.trim();
  const parsedPort = port ? Number(port) : null;
  const nowIso = (opts.now ?? new Date()).toISOString();
  return {
    id: form.id ?? opts.id ?? newConnectionId(),
    name: form.name.trim(),
    kind: form.kind,
    host: net ? form.host.trim() || null : null,
    port: net && Number.isInteger(parsedPort) ? parsedPort : null,
    database: form.database.trim(),
    username: net ? form.username.trim() || null : null,
    ssl_mode: net ? form.sslMode : "disable",
    allow_write: form.allowWrite,
    production_hint:
      form.productionHint === "yes" ? true : form.productionHint === "no" ? false : null,
    enabled: form.enabled,
    created_at: nowIso,
    updated_at: nowIso,
  };
}

/** 连接列表展示文案：只拼 kind / 主机 / 库名，**永不**拼凭据或完整 DSN。 */
export function connectionLabel(cfg: {
  kind: DbKind;
  host?: string | null;
  port?: number | null;
  database: string;
}): string {
  if (cfg.kind === "sqlite") return `SQLite · ${cfg.database}`;
  const host = (cfg.host || "").trim();
  const port = cfg.port ? `:${cfg.port}` : "";
  return `${kindLabel(cfg.kind)} · ${host}${port} · ${cfg.database}`;
}

export function utf8Bytes(text: string): number {
  return new TextEncoder().encode(text).length;
}

export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n < 0) return "-";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MiB`;
}

export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return "-";
  if (ms < 1000) return `${Math.round(ms)} ms`;
  return `${(ms / 1000).toFixed(2)} s`;
}

export function truncateText(text: string, max: number): string {
  if (max <= 0) return "";
  return text.length > max ? `${text.slice(0, max)}…` : text;
}

/**
 * 解码后端 `DbValue`（`rename_all="snake_case"` 带标签枚举）：
 * `"null"` / `{ bool }` / `{ int }` / `{ float }` / `{ text }` / `{ blob_len }`。
 * 二进制**只回长度不回字节**，统一展示为 `<binary N B>`。
 * 同时兜底「后端改为无标签标量」的情况，避免 IPC 形态变化时前端白屏。
 *
 * BUG-HUNT B8-1：此前按 PascalCase（"Null" / { Text }）解码，与后端 snake_case 实际输出不符，
 * 每个带标签单元格都命中末尾兜底 `JSON.stringify`，被渲染成原始 JSON。现按权威 DTO 对齐。
 */
export function decodeDbValue(value: unknown): DbScalar {
  if (value === null || value === undefined) return null;
  // 单元变体 Null：后端序列化为 JSON 字符串 "null"，须先于通用 string 分支判为 JS null
  if (value === "null") return null;
  if (typeof value === "boolean" || typeof value === "number" || typeof value === "string") return value;
  if (typeof value === "object") {
    const rec = value as Record<string, unknown>;
    if (typeof rec.text === "string") return rec.text;
    // database::DbValue is the live query DTO; domain's older int/float shape remains readable.
    if (typeof rec.i64 === "number") return rec.i64;
    if (typeof rec.f64 === "number") return rec.f64;
    if (rec.binary && typeof rec.binary === 'object' && typeof (rec.binary as {bytes?:unknown}).bytes === 'number') return `<binary ${formatBytes((rec.binary as {bytes:number}).bytes)}>`;
    if (typeof rec.int === "number") return rec.int;
    if (typeof rec.float === "number") return rec.float;
    if (typeof rec.bool === "boolean") return rec.bool;
    if (typeof rec.blob_len === "number") return `<binary ${formatBytes(rec.blob_len)}>`;
  }
  return JSON.stringify(value);
}

/** 单元格展示：null 显式为 NULL，字符串按展示宽度截断（截断只影响显示，不改数据）。 */
export function formatCellValue(value: unknown, max = 200): string {
  const scalar = decodeDbValue(value);
  if (scalar === null) return "NULL";
  if (typeof scalar === "boolean") return scalar ? "true" : "false";
  if (typeof scalar === "number") return String(scalar);
  return truncateText(scalar, max);
}

export function limitKindLabel(kind: DbLimitKind | null | undefined): string {
  if (kind === "rows") return "行数上限";
  if (kind === "bytes") return "字节上限";
  if (kind === "field") return "单字段上限";
  return "未命中上限";
}

export function stateLabel(state: DbQueryState): string {
  switch (state) {
    case "completed":
      return "已完成";
    case "cancelled":
      return "已取消";
    case "timeout":
      return "超时";
    case "failed":
      return "执行失败";
    default:
      return "状态未知";
  }
}

export function buildResultView(result: Partial<DbResultSet> | null | undefined): DbResultView {
  const columns = Array.isArray(result?.columns) ? result!.columns : [];
  const rawRows = Array.isArray(result?.rows) ? result!.rows : [];
  const rows = rawRows.map((row) => (Array.isArray(row) ? row : []).map((cell) => formatCellValue(cell)));
  const rowCount = Number.isFinite(result?.row_count) ? Number(result!.row_count) : rows.length;
  const truncated = !!result?.truncated;
  const fieldTruncated = !!result?.field_truncated;
  const limitHit = (result?.limit_hit ?? null) as DbLimitKind | null;
  const state = (result?.state ?? "completed") as DbQueryState;

  const warnings: string[] = [];
  if (truncated) {
    warnings.push(
      `结果已被截断（命中${limitKindLabel(limitHit)}：${DB_LIMITS.maxRows} 行 / ${formatBytes(
        DB_LIMITS.maxResultBytes,
      )}），展示的不是完整结果集`,
    );
  }
  if (fieldTruncated) {
    warnings.push(`存在超过 ${formatBytes(DB_LIMITS.maxTextFieldBytes)} 的长字段被截断`);
  }
  if (state === "cancelled") warnings.push("查询已被取消，结果不完整");
  if (state === "timeout") warnings.push("查询超时被终止，结果不完整");
  if (state === "failed") warnings.push("查询执行失败，结果不完整");

  return {
    columns,
    rows,
    rowCount,
    truncated,
    fieldTruncated,
    limitHit,
    state,
    elapsedLabel: formatDuration(Number(result?.elapsed_ms) || 0),
    rowLabel: `${rowCount} 行${truncated ? "（已截断）" : ""}`,
    stateLabel: stateLabel(state),
    risk: (result?.risk ?? "unknown") as DbRiskLevel,
    verdict: (result?.production_verdict ?? "Unknown") as ProductionVerdict,
    warnings,
  };
}

/** CSV 复制（仅复制到剪贴板/本地文本；**不写成果库**，M4-1.c 的 16 MiB 导出上限暂无消费方）。 */
export function toCsv(columns: string[], rows: unknown[][]): string {
  const esc = (v: unknown): string => {
    const scalar = decodeDbValue(v);
    if (scalar === null) return "";
    const s = typeof scalar === "boolean" ? (scalar ? "true" : "false") : typeof scalar === "number" ? String(scalar) : scalar;
    return /[",\r\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  };
  const head = columns.map((c) => esc(c)).join(",");
  const body = rows.map((r) => (Array.isArray(r) ? r : []).map((c) => esc(c)).join(","));
  return [head, ...body].join("\n");
}

export function sqlIssues(sql: string): DbIssue[] {
  const issues: DbIssue[] = [];
  if (!sql.trim()) issues.push({ field: "sql", message: "SQL 不能为空" });
  if (utf8Bytes(sql) > DB_LIMITS.maxSqlBytes) {
    issues.push({ field: "sql", message: `SQL 超过 ${formatBytes(DB_LIMITS.maxSqlBytes)} 上限` });
  }
  return issues;
}

/**
 * 多语句**提示**（仅本地提示，不作判定）：后端 `classify_sql_risk` 才是唯一判定位（F4），
 * 前端不得据此放行或拒绝，只用于提醒用户后端会拒多语句。
 */
export function looksLikeMultipleStatements(sql: string): boolean {
  const body = sql.trim().replace(/;\s*$/, "");
  return body.includes(";");
}

export function riskLabel(level: DbRiskLevel): string {
  switch (level) {
    case "read":
      return "只读";
    case "write":
      return "写操作";
    case "ddl":
      return "结构变更";
    case "admin":
      return "权限/运维";
    default:
      return "未知（按最严处理）";
  }
}

export function productionLabel(verdict: ProductionVerdict): string {
  if (verdict === "Production") return "生产环境";
  if (verdict === "NonProduction") return "非生产环境";
  return "生产判定未知（按生产处理）";
}

/** 非只读语句一律需二次确认（M4-1.d §3：写放行必须过确认）。 */
export function needsWriteConfirm(risk: DbRiskLevel): boolean {
  return risk !== "read";
}

/** 写被拒原因（null = 后端可放行；写操作后端还会再判一次，前端不做分类决策）。 */
export function writeBlockedReason(
  risk: DbRiskLevel,
  allowWrite: boolean,
  verdict: ProductionVerdict,
): string | null {
  if (risk === "read") return null;
  if (!allowWrite) return "连接未开启写权限（写默认拒绝，F1）";
  if (verdict !== "NonProduction") return `生产判定为${productionLabel(verdict)}，拒绝写（Unknown 按生产处理）`;
  return null;
}

export interface RunGate {
  backendReady: boolean;
  connected: boolean;
  sql: string;
}

/** 运行前置门禁：任一不满足即不发 IPC。 */
export function canRunQuery(gate: RunGate): { ok: boolean; reason: string } {
  if (!gate.backendReady) return { ok: false, reason: "后端数据库命令未就绪" };
  if (!gate.connected) return { ok: false, reason: "尚未连接到数据库" };
  const issues = sqlIssues(gate.sql);
  if (issues.length) return { ok: false, reason: issues[0].message };
  return { ok: true, reason: "" };
}
