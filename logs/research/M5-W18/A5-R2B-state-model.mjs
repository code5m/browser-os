#!/usr/bin/env node
// A5-R2B 合成状态模型（零产品依赖、纯研究资产）
// ---------------------------------------------------------------------------
// 目的：演示 R2B §6「A5 验收」要求的核心不变量，作为 J2 测试脚本与实现卡的前置证据：
//   (1) 至少两份 SQL 文档相互独立；
//   (2) 查询 → 结果 → 取消 → 重试 完整；
//   (3) 切换文档时，旧请求的迟到的结果不得覆盖新文档（execId 陈旧守卫）；
//   (4) 取消幂等。
// 本文件不 import 任何 src/ 代码，仅以纯函数表达「将落入 useDatabaseStore.ts / dbUi.ts」的
// 状态契约，便于编码 lane 直接移植并加进 check-database-ui-logic.mjs。
//
// 分类：REIMPLEMENT_FROM_BEHAVIOR
//   参考 dbx apps/desktop/src/stores/queryStore.ts:105-106（CANCEL_QUERY_TIMEOUT_MS=10_000 /
//   CANCEL_ACK_SETTLE_TIMEOUT_MS=2_000）与 TabExecutionStatus 思路；按 Vue/Pinia 自实现，
//   不拷贝 324KB 的 queryStore.ts。

// ---- 纯函数状态契约（设计参考实现） ----

/** 新建一个 SQL 文档（对应 WORKBENCH_BLUEPRINT §5「文档」契约：身份键/类型/所属/状态）。 */
export function createSqlDoc(id, title) {
  return {
    id,
    title,
    sql: "",
    connId: null,
    execId: null, // 当前在途请求的执行 ID；null = 无在途
    seq: 0, // 单调增，用于陈旧判定（与 execId 等价，二者任一即可）
    status: "idle", // idle|running|success|error|cancelled|timeout
    result: null,
    error: null,
  };
}

/** 发起运行：登记新 execId，置 running。返回新文档（不可变更新）。 */
export function startRun(doc, sql, connId, makeExecId) {
  if (doc.status === "running") return doc; // 已有在途，忽略重复触发
  const execId = makeExecId();
  return { ...doc, sql, connId, execId, seq: doc.seq + 1, status: "running", error: null };
}

/**
 * 应用结果：仅当 execId 匹配才提交，否则原样返回（陈旧守卫）。
 * 这是「切换文档旧请求不得覆盖新文档」的核心保证。
 */
export function applyDocResult(doc, execId, result, error) {
  if (doc.execId !== execId) return doc; // 陈旧响应：丢弃
  return {
    ...doc,
    result: result ?? null,
    error: error ?? null,
    status: error ? "error" : result && result.state === "timeout" ? "timeout"
      : result && result.state === "cancelled" ? "cancelled" : "success",
  };
}

/** 取消：仅对在途有效；幂等（已非 running 则原样返回）。 */
export function cancelDoc(doc) {
  if (doc.status !== "running") return doc;
  return { ...doc, status: "cancelled", execId: null };
}

// ---- 简易断言框架 ----
let failures = 0;
function check(name, cond) {
  if (cond) console.log("  ok   " + name);
  else { console.log("  FAIL " + name); failures++; }
}

// ---- 场景 ----

// 场景 1：两份文档相互独立
{
  let a = createSqlDoc("a", "console-a.sql");
  let b = createSqlDoc("b", "console-b.sql");
  const idGen = (() => { let n = 0; return () => "exec-" + (++n); })();

  a = startRun(a, "select 1", "conn1", idGen); // a 在途，execId=exec-1
  const aExec = a.execId;
  const b0 = b; // b 仍未动

  // a 的结果到达
  a = applyDocResult(a, aExec, { state: "completed", rows: [[1]], row_count: 1 });
  check("S1: doc A 收到结果后置 success", a.status === "success" && a.result.row_count === 1);
  check("S1: doc B 完全未受影响（独立性）", b === b0 && b.status === "idle" && b.execId === null);
}

// 场景 2：切换文档后，旧请求迟到结果不得覆盖新文档（execId 陈旧守卫）
{
  let cur = createSqlDoc("x", "x.sql");
  let other = createSqlDoc("y", "y.sql");
  const idGen = (() => { let n = 0; return () => "e" + (++n); })();

  cur = startRun(cur, "select slow", "c1", idGen); // cur.execId = e1
  const staleId = cur.execId;

  // 用户切换到 other 并在 other 上发起新查询
  other = startRun(other, "select fast", "c1", idGen); // other.execId = e2

  // 旧请求（针对 cur，execId=e1）迟到，但此时我们正要把结果写到 other——
  // 关键：applyDocResult 用的是「目标文档当前的 execId」，陈旧响应被丢弃。
  const lateForOther = applyDocResult(other, staleId, { state: "completed", rows: [["OLD"]] });
  check("S2: 陈旧 execId(e1) 对 other 被丢弃，other 仍 running", lateForOther === other && other.status === "running");

  // other 自己的结果到达
  other = applyDocResult(other, other.execId, { state: "completed", rows: [["NEW"]] });
  check("S2: other 收到正确结果", other.status === "success" && other.result.rows[0][0] === "NEW");
}

// 场景 3：取消幂等 + 重试
{
  let d = createSqlDoc("z", "z.sql");
  const idGen = (() => { let n = 0; return () => "r" + (++n); })();
  d = startRun(d, "select 2", "c1", idGen); // running
  check("S3: 运行态可取消", d.status === "running");
  const cancelled = cancelDoc(d);
  check("S3: 取消后置 cancelled", cancelled.status === "cancelled");
  const again = cancelDoc(cancelled);
  check("S3: 重复取消幂等（无变化）", again === cancelled && again.status === "cancelled");

  // 重试：新 execId 重新发起
  d = cancelled;
  d = startRun(d, "select 2", "c1", idGen);
  check("S3: cancelled 后可重试并获新 execId", d.status === "running" && d.execId !== null);
}

// 场景 4：截断提示沿用 dbUi.buildResultView（此处仅断言守卫不破坏截断字段透传）
{
  let d = createSqlDoc("t", "t.sql");
  const idGen = (() => { let n = 0; return () => "q" + (++n); })();
  d = startRun(d, "select big", "c1", idGen);
  const res = { state: "completed", truncated: true, field_truncated: true, limit_hit: "rows", row_count: 1000, rows: [] };
  d = applyDocResult(d, d.execId, res);
  check("S4: 截断/字段截断标志透传（交由 dbUi.buildResultView 渲染告警）",
    d.status === "success" && d.result.truncated === true && d.result.field_truncated === true);
}

console.log(failures === 0 ? "\nA5-R2B state-model: ALL_PASS" : `\nA5-R2B state-model: ${failures} FAIL`);
process.exit(failures === 0 ? 0 : 1);
