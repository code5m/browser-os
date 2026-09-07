#!/usr/bin/env node
// M4-4 数据库面板 UI 逻辑层自动化测试（headless，无 GUI 依赖）。
// 加载**真实** src/utils/dbUi.ts（与 check-command-ui-logic.mjs 同一范式），不重实现逻辑。

import * as nodeModule from "node:module";

function resolveWithExt(specifier, context, next) {
  try {
    return next(specifier, context);
  } catch (err) {
    if (specifier.startsWith(".") || specifier.startsWith("/")) {
      for (const ext of [".ts", "/index.ts", ".mjs", ".js"]) {
        try {
          return next(specifier + ext, context);
        } catch {}
      }
    }
    throw err;
  }
}

if (typeof nodeModule.registerHooks === "function") {
  nodeModule.registerHooks({ resolve: resolveWithExt });
} else {
  nodeModule.register(
    "data:text/javascript," +
      encodeURIComponent(
        `export async function resolve(specifier, context, next) {
  return globalThis.__dbUiResolve(specifier, context, next);
}`,
      ),
  );
  globalThis.__dbUiResolve = resolveWithExt;
}

const ROOT = new URL("..", import.meta.url).pathname;
const {
  DB_LIMITS,
  buildConnectPayload,
  buildResultView,
  canRunQuery,
  connectionLabel,
  databaseFieldLabel,
  decodeDbValue,
  defaultPortFor,
  emptyConnectionForm,
  formatBytes,
  formatCellValue,
  formatDuration,
  kindLabel,
  kindNeedsCredentials,
  limitKindLabel,
  looksLikeMultipleStatements,
  needsWriteConfirm,
  productionLabel,
  riskLabel,
  sqlIssues,
  toCsv,
  truncateText,
  utf8Bytes,
  validateConnectionForm,
  visibleFields,
  writeBlockedReason,
} = await import(`${ROOT}src/utils/dbUi.ts`);

let passed = 0;
const failures = [];

function eq(actual, expected, label) {
  if (JSON.stringify(actual) === JSON.stringify(expected)) passed += 1;
  else failures.push(`${label}\n    期望: ${JSON.stringify(expected)}\n    实际: ${JSON.stringify(actual)}`);
}

function ok(cond, label) {
  if (cond) passed += 1;
  else failures.push(label);
}

// ===== 1. 表单缺省值（F1：写默认拒绝） =====
const empty = emptyConnectionForm();
eq(empty.allowWrite, false, "新建连接默认关闭写权限（F1 写默认拒绝）");
eq(empty.sslMode, "prefer", "SSL 缺省 prefer（与 DbSslMode::default 一致）");
eq(empty.productionHint, "", "生产标记缺省未标记（production_hint=null）");
eq(empty.enabled, true, "新建连接默认启用");
eq(empty.kind, "sqlite", "默认驱动 sqlite");
ok(!("password" in empty), "连接表单结构性无 password 字段（F2）");

// ===== 2. 驱动差异 =====
eq(kindLabel("mysql"), "MySQL", "驱动展示名映射");
eq(defaultPortFor("mysql"), 3306, "MySQL 默认端口");
eq(defaultPortFor("postgres"), 5432, "PostgreSQL 默认端口");
eq(defaultPortFor("sqlite"), null, "SQLite 无默认端口");
eq(kindNeedsCredentials("sqlite"), false, "SQLite 不需要网络凭据");
eq(kindNeedsCredentials("postgres"), true, "PostgreSQL 需要网络凭据");
eq(databaseFieldLabel("sqlite"), "文件路径", "SQLite 字段标注为文件路径");
eq(databaseFieldLabel("mysql"), "数据库名", "MySQL 字段标注为数据库名");
eq(visibleFields("sqlite"), { host: false, port: false, username: false, ssl: false }, "SQLite 隐藏网络字段");
eq(visibleFields("mysql"), { host: true, port: true, username: true, ssl: true }, "MySQL 展示网络字段");

// ===== 3. 连接表单校验 =====
const validSqlite = { ...empty, name: "local", database: "/data/app.db" };
eq(validateConnectionForm(validSqlite).length, 0, "合法 SQLite 连接零校验问题");
const validMysql = {
  ...empty,
  name: "prod-ro",
  kind: "mysql",
  host: "127.0.0.1",
  portText: "3306",
  database: "app",
  username: "reader",
};
eq(validateConnectionForm(validMysql).length, 0, "合法 MySQL 连接零校验问题");

ok(validateConnectionForm({ ...validSqlite, name: "" }).some((i) => i.field === "name"), "空连接名应报错");
ok(validateConnectionForm({ ...validSqlite, database: "  " }).some((i) => i.field === "database"), "空文件路径应报错");
ok(validateConnectionForm({ ...validMysql, host: "" }).some((i) => i.field === "host"), "空主机应报错");
ok(validateConnectionForm({ ...validMysql, username: "" }).some((i) => i.field === "username"), "空用户名应报错");
ok(validateConnectionForm({ ...validMysql, portText: "70000" }).some((i) => i.field === "portText"), "越界端口应报错");
ok(validateConnectionForm({ ...validMysql, portText: "abc" }).some((i) => i.field === "portText"), "非数字端口应报错");
eq(validateConnectionForm({ ...validMysql, portText: "" }).length, 0, "端口留空走默认端口");

// ===== 4. payload 构造（F2：凭据零落地） =====
const payloadSqlite = buildConnectPayload(validSqlite);
eq(payloadSqlite.allow_write, false, "payload 默认 allow_write=false");
eq(payloadSqlite.host, null, "SQLite payload 的 host 为 null");
eq(payloadSqlite.username, null, "SQLite payload 的 username 为 null");
eq(payloadSqlite.ssl_mode, "disable", "SQLite payload 的 ssl_mode 为 disable");
eq(payloadSqlite.production_hint, null, "未标记生产时 production_hint 为 null");
ok(!Object.keys(payloadSqlite).some((k) => /pass|dsn|connection_string|credential/i.test(k)), "payload 不得出现密码/DSN 类键（F2）");

const payloadMysql = buildConnectPayload({ ...validMysql, allowWrite: true, productionHint: "yes" });
eq(payloadMysql.allow_write, true, "payload 保留 allow_write=true");
eq(payloadMysql.production_hint, true, "productionHint=yes 映射为 true");
eq(buildConnectPayload({ ...validMysql, productionHint: "no" }).production_hint, false, "productionHint=no 映射为 false");
eq(payloadMysql.port, 3306, "payload 保留数值端口");
eq(payloadMysql.ssl_mode, "prefer", "payload 保留 ssl_mode");
ok(!Object.keys(payloadMysql).some((k) => /pass|dsn|connection_string|credential/i.test(k)), "MySQL payload 同样无凭据键（F2）");

const passwordArg = "p@ss/secret";
const withTransient = { ...buildConnectPayload(validMysql), password: passwordArg };
ok(!("password" in buildConnectPayload(validMysql)), "buildConnectPayload 本身不接受/不落地密码");
ok("password" in withTransient, "密码只作为调用方瞬时附加参数存在，不进表单与 payload 构造");

// ===== 5. 展示文案脱敏 =====
const label = connectionLabel({ kind: "mysql", host: "db.internal", port: 3306, database: "app" });
ok(label.includes("db.internal") && label.includes("app"), "连接标签含主机与库名");
ok(!label.includes("@"), "连接标签不含 user:pass@ 形态");
ok(connectionLabel({ kind: "sqlite", database: "/data/a.db" }).includes("/data/a.db"), "SQLite 标签含文件路径");

// ===== 6. 格式化 =====
eq(utf8Bytes("a"), 1, "ASCII 字节长度");
eq(utf8Bytes("中"), 3, "中文按 UTF-8 计 3 字节");
eq(formatBytes(512), "512 B", "字节格式化 B");
eq(formatBytes(2048), "2.0 KiB", "字节格式化 KiB");
eq(formatBytes(4 * 1024 * 1024), "4.00 MiB", "字节格式化 MiB");
eq(formatDuration(120), "120 ms", "毫秒格式化");
eq(formatDuration(1500), "1.50 s", "秒格式化");
eq(truncateText("abcdef", 3), "abc…", "长文本截断并带省略号");
eq(truncateText("abc", 3), "abc", "未超长不截断");
eq(formatCellValue(null), "NULL", "null 展示为 NULL");
eq(formatCellValue(true), "true", "布尔展示");
eq(formatCellValue(12.5), "12.5", "数值展示");
eq(formatCellValue("x".repeat(300)).endsWith("…"), true, "超长单元格展示截断");

// ===== 7. 结果视图与截断（M4-1.c §1 规则 2：禁止静默截断） =====
const full = buildResultView({
  columns: ["id", "name"],
  rows: [[1, "a"], [2, null]],
  row_count: 2,
  truncated: false,
  field_truncated: false,
  elapsed_ms: 12,
  query_id: "q1",
});
eq(full.columns, ["id", "name"], "结果视图保留列名");
eq(full.rows[1][1], "NULL", "结果视图 null 单元格展示为 NULL");
eq(full.rowCount, 2, "结果视图行数");
eq(full.elapsedLabel, "12 ms", "结果视图耗时展示");
eq(full.warnings.length, 0, "未截断时无告警");
eq(full.rowLabel, "2 行", "未截断行数文案");

const cut = buildResultView({
  columns: ["id"],
  rows: [[1]],
  row_count: DB_LIMITS.maxRows,
  truncated: true,
  field_truncated: true,
  elapsed_ms: 0,
  query_id: null,
});
ok(cut.warnings.length >= 2, "截断 + 字段截断都要有告警");
ok(cut.rowLabel.includes("已截断"), "行数文案显式标注已截断");
ok(cut.warnings.some((w) => w.includes("不是完整结果集")), "截断告警说明结果不完整");
eq(buildResultView(null).warnings.length, 0, "空结果不抛异常");
eq(buildResultView(null).rowCount, 0, "空结果行数为 0");

// ===== 7b. 后端 DbValue 带标签解码（domain.rs rename_all="snake_case"） =====
// BUG-HUNT B8-1：断言此前按 PascalCase 书写（与后端 snake_case 实际输出不符，等于把缺陷
// 固化为契约）。现按权威 DTO 对齐 snake_case；任何回退到 PascalCase 的改动都会在此失败。
eq(decodeDbValue("null"), null, "字符串 null 解码为 null");
eq(decodeDbValue({ bool: true }), true, "bool 标签解码为布尔");
eq(decodeDbValue({ int: 7 }), 7, "int 标签解码为数值");
eq(decodeDbValue({ float: 1.5 }), 1.5, "float 标签解码为数值");
eq(decodeDbValue({ text: "abc" }), "abc", "text 标签解码为字符串");
eq(formatCellValue({ text: "abc" }), "abc", "text 单元格原文展示");
eq(formatCellValue({ int: 42 }), "42", "int 单元格展示");
eq(formatCellValue({ bool: false }), "false", "bool 单元格展示");
eq(formatCellValue("null"), "NULL", "null 单元格展示为 NULL");
eq(formatCellValue({ blob_len: 1024 }), "<binary 1.0 KiB>", "二进制列只展示长度不回字节");
eq(formatCellValue({ Weird: 1 }), JSON.stringify({ Weird: 1 }), "未知形态安全降级为 JSON 文本");

// ===== 7c. 状态与上限命中（非静默） =====
eq(limitKindLabel("rows"), "行数上限", "行数上限文案");
eq(limitKindLabel("bytes"), "字节上限", "字节上限文案");
eq(limitKindLabel("field"), "单字段上限", "单字段上限文案");
eq(limitKindLabel(null), "未命中上限", "未命中上限文案");
const cancelled = buildResultView({ columns: ["a"], rows: [], row_count: 0, truncated: false, field_truncated: false, elapsed_ms: 0, state: "cancelled", query_id: "q" });
eq(cancelled.stateLabel, "已取消", "取消状态文案");
ok(cancelled.warnings.some((w) => w.includes("取消") && w.includes("不完整")), "取消必须给出结果不完整告警");
const timedOut = buildResultView({ columns: ["a"], rows: [], row_count: 0, truncated: false, field_truncated: false, elapsed_ms: 0, state: "timeout", query_id: "q" });
ok(timedOut.warnings.some((w) => w.includes("超时")), "超时必须给出结果不完整告警");
const failed = buildResultView({ columns: ["a"], rows: [], row_count: 0, truncated: false, field_truncated: false, elapsed_ms: 0, state: "failed", query_id: "q" });
ok(failed.warnings.some((w) => w.includes("失败")), "失败必须给出结果不完整告警");
const limitRows = buildResultView({ columns: ["a"], rows: [[{ int: 1 }]], row_count: 1000, truncated: true, field_truncated: false, limit_hit: "rows", elapsed_ms: 5, state: "completed", query_id: "q" });
eq(limitRows.rows[0][0], "1", "带标签值在结果视图中解码展示");
ok(limitRows.warnings.some((w) => w.includes("行数上限")), "命中行数上限时告警要明确原因");
eq(buildResultView({ columns: [], rows: [], row_count: 0, truncated: false, field_truncated: false, elapsed_ms: 0, state: "completed", query_id: "q", risk: "read", production_verdict: "NonProduction" }).verdict, "NonProduction", "结果视图回带后端生产判定");
eq(buildResultView({ columns: [], rows: [], row_count: 0, truncated: false, field_truncated: false, elapsed_ms: 0, state: "completed", query_id: "q" }).risk, "unknown", "后端未回带风险时按 unknown 处理");

// ===== 8. CSV 复制 =====
eq(toCsv(["a", "b"], [[1, "x,y"]]), "a,b\n1,\"x,y\"", "含逗号的单元格加引号");
eq(toCsv(["a"], [["he said \"hi\""]]), "a\n\"he said \"\"hi\"\"\"", "引号转义为双引号");
eq(toCsv(["a"], [["l1\nl2"]]), "a\n\"l1\nl2\"", "含换行的单元格加引号");
eq(toCsv(["a"], [[null]]), "a\n", "null 导出为空");
eq(toCsv([], []), "", "空结果导出空串");
eq(toCsv(["a"], [[{ text: "x,y" }]]), "a\n\"x,y\"", "带标签值同样按 CSV 转义");
eq(toCsv(["a"], [["null"], [null]]), "a\n\n", "字符串 null 与 JS null 都导出为空");
eq(toCsv(["a"], [[{ blob_len: 128 }]]), "a\n<binary 128 B>", "二进制列导出为长度占位而非字节");

// ===== 9. SQL 前置校验与提示 =====
eq(sqlIssues("select 1").length, 0, "正常 SQL 无问题");
ok(sqlIssues("   ").some((i) => i.field === "sql"), "空 SQL 应报错");
ok(sqlIssues("x".repeat(DB_LIMITS.maxSqlBytes + 1)).some((i) => i.message.includes("上限")), "超长 SQL 应报错");
eq(looksLikeMultipleStatements("select 1; select 2"), true, "多语句提示为真");
eq(looksLikeMultipleStatements("select 1;"), false, "仅结尾分号不算多语句");

// ===== 10. 风险与写闸门（fail-closed） =====
eq(riskLabel("read"), "只读", "只读风险文案");
eq(riskLabel("ddl"), "结构变更", "DDL 风险文案");
eq(riskLabel("admin"), "权限/运维", "Admin 风险文案（对应 SqlRiskClass::Admin）");
eq(riskLabel("unknown"), "未知（按最严处理）", "未知风险按最严展示");
eq(productionLabel("Unknown"), "生产判定未知（按生产处理）", "Unknown 生产判定文案");
eq(needsWriteConfirm("read"), false, "只读无需二次确认");
eq(needsWriteConfirm("write"), true, "写操作需二次确认");
eq(needsWriteConfirm("unknown"), true, "未知风险需二次确认");
eq(writeBlockedReason("read", false, "Unknown"), null, "只读不受写闸门约束");
ok(writeBlockedReason("write", false, "NonProduction") !== null, "未开写权限时写被拒");
ok(writeBlockedReason("write", true, "Unknown") !== null, "生产判定 Unknown 时写被拒（fail-closed）");
ok(writeBlockedReason("write", true, "Production") !== null, "生产库写被拒");
eq(writeBlockedReason("write", true, "NonProduction"), null, "非生产 + 已开写权限可放行");

// ===== 11. 运行门禁（未就绪不发 IPC） =====
eq(canRunQuery({ backendReady: false, connected: true, sql: "select 1" }).ok, false, "后端未就绪不得运行");
eq(canRunQuery({ backendReady: true, connected: false, sql: "select 1" }).ok, false, "未连接不得运行");
eq(canRunQuery({ backendReady: true, connected: true, sql: "  " }).ok, false, "空 SQL 不得运行");
eq(canRunQuery({ backendReady: true, connected: true, sql: "select 1" }).ok, true, "就绪且已连接可运行");

if (failures.length === 0) {
  console.log(`check-database-ui-logic: ${passed} 断言全部通过`);
  process.exit(0);
}
console.error(`check-database-ui-logic: ${failures.length} 项失败（通过 ${passed}）`);
for (const f of failures) console.error(`  x ${f}`);
process.exit(1);
